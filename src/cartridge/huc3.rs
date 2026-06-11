use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// HuC3 (cart type 0xFE) — MBC1-style ROM/RAM banking with RTC and IR.
///
/// The 0x0000-0x1FFF register selects an operating mode:
///   0x0A = RAM access  (normal read/write through cartridge RAM)
///   0x0B = RTC command (game writes nibbles to 0xA000)
///   0x0C = RTC read    (game reads response nibbles from 0xA000)
///   0x0D = IR mode     — stub (reads 0x01, writes ignored)
///   other = disabled   (0xFF on read)
///
/// RTC nibble protocol (mode 0x0B writes, mode 0x0C reads):
///   Each write to 0xA000 sends one nibble (lower 4 bits of the written byte).
///   Reads return 0x01 (device ready) when no response is pending.
///
///   [0x1, 0x1]              → latch current time; respond with 7 nibbles in mode 0x0C
///   [0x1, 0x3, d0..d6]     → set time from 7 data nibbles
///
///   The 7 time nibbles are: [min_lo, min_hi, hour_lo, hour_hi, day_0, day_1, day_2]
///   where min/hour are BCD and day is a 12-bit counter (≈11 years before wrap).
#[derive(Serialize, Deserialize)]
pub struct CartridgeHuC3 {
    pub(super) cart: Cartridge,
    mode: u8,

    // unix epoch at which the RTC counter equals zero (same concept as MBC3)
    pub(super) rtc_base_unix_secs: u64,

    // Incoming command nibble buffer.  Layout:
    //   [0]    = first command nibble
    //   [1]    = sub-command nibble (determines read vs write)
    //   [2..9] = up to 7 data nibbles (write-time only)
    cmd_buf: [u8; 9],
    cmd_len: u8,

    // Outgoing response nibble buffer (populated by latch_time).
    resp_buf: [u8; 7],
    resp_len: u8,
    // Response cursor.  Cell for interior mutability: read_ram takes &self.
    resp_pos: Cell<u8>,
}

impl CartridgeHuC3 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            mode: 0,
            rtc_base_unix_secs: now_unix_secs(),
            cmd_buf: [0; 9],
            cmd_len: 0,
            resp_buf: [0; 7],
            resp_len: 0,
            resp_pos: Cell::new(0),
        }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                self.mode = byte & 0x0F;
                if let Err(e) = self.cart.update_ram_enabled(self.mode == 0x0A) {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                self.cart.rom_bank = ((byte & 0x7F) as u16).max(1);
            }
            0x4000 | 0x5000 => {
                self.cart.ram_bank = byte & 0x0F;
            }
            0x6000 | 0x7000 => {}
            _ => panic!("Unhandled HuC3 ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        match self.mode {
            0x0A => self.cart.read_ram(addr),
            0x0B => 0x01, // device ready; command-mode reads don't return data nibbles
            0x0C => {
                let pos = self.resp_pos.get();
                if pos < self.resp_len {
                    self.resp_pos.set(pos + 1);
                    self.resp_buf[pos as usize]
                } else {
                    0x01 // idle / no more data
                }
            }
            _ => 0xFF,
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.mode == 0x0A {
            self.cart.write_ram(addr, byte);
        } else if self.mode == 0x0B {
            self.nibble_in(byte & 0x0F);
        }
        // mode 0x0C and IR (0x0D) writes are ignored
    }

    // ---- RTC command state machine ----

    fn nibble_in(&mut self, nibble: u8) {
        if (self.cmd_len as usize) < self.cmd_buf.len() {
            self.cmd_buf[self.cmd_len as usize] = nibble;
            self.cmd_len += 1;
        }

        match (self.cmd_buf[0], self.cmd_buf[1], self.cmd_len) {
            // [0x1, 0x1] — read time
            (0x1, 0x1, 2) => {
                self.latch_time();
                self.cmd_len = 0;
            }
            // [0x1, 0x3, d0..d6] — write time (9 nibbles total)
            (0x1, 0x3, 9) => {
                let d: [u8; 7] = self.cmd_buf[2..9].try_into().unwrap();
                self.apply_write_time(&d);
                self.cmd_len = 0;
            }
            _ => {}
        }
    }

    // Decompose elapsed time and fill resp_buf with 7 nibbles.
    fn latch_time(&mut self) {
        let elapsed = now_unix_secs().saturating_sub(self.rtc_base_unix_secs);
        let mins = (elapsed / 60) % 60;
        let hours = (elapsed / 3600) % 24;
        // 12-bit day counter wraps after 4095 days (~11 years)
        let days = (elapsed / 86400) & 0xFFF;

        self.resp_buf = [
            (mins % 10) as u8,         // minutes BCD low
            (mins / 10) as u8,         // minutes BCD high
            (hours % 10) as u8,        // hours BCD low
            (hours / 10) as u8,        // hours BCD high
            (days & 0xF) as u8,        // day bits 3:0
            ((days >> 4) & 0xF) as u8, // day bits 7:4
            ((days >> 8) & 0xF) as u8, // day bits 11:8
        ];
        self.resp_len = 7;
        self.resp_pos.set(0);
    }

    // Reconstruct elapsed seconds from 7 data nibbles and update the RTC base.
    fn apply_write_time(&mut self, d: &[u8; 7]) {
        let mins = d[0] as u64 + d[1] as u64 * 10;
        let hours = d[2] as u64 + d[3] as u64 * 10;
        let days = d[4] as u64 | (d[5] as u64) << 4 | (d[6] as u64) << 8;
        let elapsed = days * 86400 + hours * 3600 + mins * 60;
        self.rtc_base_unix_secs = now_unix_secs().saturating_sub(elapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, RAM_BANK_SIZE, ROM_BANK_SIZE};
    use std::path::PathBuf;

    fn make_huc3(num_rom_banks: usize) -> CartridgeHuC3 {
        let mut rom = vec![0u8; num_rom_banks * ROM_BANK_SIZE];
        for b in 0..num_rom_banks {
            rom[b * ROM_BANK_SIZE + 0x100] = b as u8;
        }
        let cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        CartridgeHuC3::new(cart)
    }

    #[test]
    fn mode_0a_allows_ram_readwrite() {
        let mut c = make_huc3(2);
        c.cart.ram = vec![0u8; RAM_BANK_SIZE];
        c.write_rom(0x0000, 0x0A); // set mode 0x0A, enables RAM
        c.write_ram(0, 0x55);
        assert_eq!(c.read_ram(0), 0x55);
    }

    #[test]
    fn other_modes_read_return_correct_values() {
        let mut c = make_huc3(2);
        c.cart.ram = vec![0u8; RAM_BANK_SIZE];
        c.write_rom(0x0000, 0x00); // disabled mode
        assert_eq!(c.read_ram(0), 0xFF);
        c.write_rom(0x0000, 0x0B); // RTC command mode — returns 0x01 (ready)
        assert_eq!(c.read_ram(0), 0x01);
        c.write_rom(0x0000, 0x0C); // RTC read mode — no pending data, returns 0x01
        assert_eq!(c.read_ram(0), 0x01);
    }

    #[test]
    fn write_outside_mode_0a_is_ignored() {
        let mut c = make_huc3(2);
        c.cart.ram = vec![0u8; RAM_BANK_SIZE];
        c.write_rom(0x0000, 0x0A); // enable RAM
        c.write_ram(0, 0x42);
        c.write_rom(0x0000, 0x0B); // switch away from RAM mode
        c.write_ram(0, 0xFF); // should be ignored
        c.write_rom(0x0000, 0x0A); // re-enable
        assert_eq!(c.read_ram(0), 0x42);
    }

    #[test]
    fn rom_bank_zero_clamps_to_one() {
        let mut c = make_huc3(2);
        c.write_rom(0x2000, 0x00);
        assert_eq!(c.read_rom(0x4100), 1);
    }

    #[test]
    fn rom_banking_switchable_window() {
        let mut c = make_huc3(4);
        c.write_rom(0x2000, 3);
        assert_eq!(c.read_rom(0x4100), 3);
    }

    // Send command nibbles in mode 0x0B.
    fn send_cmd(c: &mut CartridgeHuC3, nibbles: &[u8]) {
        c.write_rom(0x0000, 0x0B);
        for &n in nibbles {
            c.write_ram(0x0000, n);
        }
    }

    // Read response nibbles in mode 0x0C.
    fn read_resp(c: &mut CartridgeHuC3, count: usize) -> Vec<u8> {
        c.write_rom(0x0000, 0x0C);
        (0..count).map(|_| c.read_ram(0x0000)).collect()
    }

    #[test]
    fn rtc_read_time_command_returns_7_nibbles() {
        let mut c = make_huc3(2);
        // Set a known elapsed time: 1 day, 2 hours, 3 minutes
        let elapsed: u64 = 86400 + 2 * 3600 + 3 * 60;
        c.rtc_base_unix_secs = now_unix_secs().saturating_sub(elapsed);

        send_cmd(&mut c, &[0x1, 0x1]);
        let resp = read_resp(&mut c, 7);

        // minutes: 3 → BCD [3, 0]
        assert_eq!(resp[0], 3); // min_lo
        assert_eq!(resp[1], 0); // min_hi
        // hours: 2 → BCD [2, 0]
        assert_eq!(resp[2], 2); // hour_lo
        assert_eq!(resp[3], 0); // hour_hi
        // days: 1 → nibbles [1, 0, 0]
        assert_eq!(resp[4], 1); // day bits 3:0
        assert_eq!(resp[5], 0); // day bits 7:4
        assert_eq!(resp[6], 0); // day bits 11:8
    }

    #[test]
    fn rtc_write_then_read_roundtrip() {
        let mut c = make_huc3(2);

        // Write: 5 minutes, 10 hours, 2 days
        // mins=5 → [5, 0], hours=10 → [0, 1], days=2 → [2, 0, 0]
        send_cmd(&mut c, &[0x1, 0x3, 5, 0, 0, 1, 2, 0, 0]);
        send_cmd(&mut c, &[0x1, 0x1]);
        let resp = read_resp(&mut c, 7);

        assert_eq!(resp[0], 5); // min_lo = 5
        assert_eq!(resp[1], 0); // min_hi = 0
        assert_eq!(resp[2], 0); // hour_lo = 0 (10 % 10)
        assert_eq!(resp[3], 1); // hour_hi = 1 (10 / 10)
        assert_eq!(resp[4], 2); // day_lo
        assert_eq!(resp[5], 0);
        assert_eq!(resp[6], 0);
    }

    #[test]
    fn rtc_idle_returns_0x01_after_all_nibbles_consumed() {
        let mut c = make_huc3(2);
        send_cmd(&mut c, &[0x1, 0x1]);
        let resp = read_resp(&mut c, 8); // read one past end
        assert_eq!(resp[7], 0x01); // idle sentinel
    }
}
