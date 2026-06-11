use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Serialize, Deserialize)]
pub struct Rtc {
    pub(super) base_unix_secs: u64,
    latched: [u8; 5], // [S, M, H, DL, DH] snapshot shown to the game
    latch_saw_zero: bool,
}

impl Rtc {
    fn new() -> Self {
        Self {
            base_unix_secs: now_unix_secs(),
            latched: [0; 5],
            latch_saw_zero: false,
        }
    }

    fn latch(&mut self) {
        let elapsed = now_unix_secs().saturating_sub(self.base_unix_secs);
        let secs = (elapsed % 60) as u8;
        let mins = ((elapsed / 60) % 60) as u8;
        let hours = ((elapsed / 3600) % 24) as u8;
        let days = elapsed / 86400;
        let days_lo = (days & 0xFF) as u8;
        let mut days_hi: u8 = 0;
        if days & 0x100 != 0 {
            days_hi |= 0x01;
        }
        if days >= 512 {
            days_hi |= 0x80;
        }
        self.latched = [secs, mins, hours, days_lo, days_hi];
    }

    fn read(&self, reg: u8) -> u8 {
        self.latched[reg as usize]
    }

    // Update base so the clock reflects the new register value.
    fn write(&mut self, reg: u8, val: u8) {
        self.latch();
        self.latched[reg as usize] = val;
        let s = self.latched[0] as u64;
        let m = self.latched[1] as u64;
        let h = self.latched[2] as u64;
        let dl = self.latched[3] as u64;
        let dh = self.latched[4] as u64;
        let days = dl | ((dh & 0x01) << 8);
        let total = days * 86400 + h * 3600 + m * 60 + s;
        self.base_unix_secs = now_unix_secs().saturating_sub(total);
    }
}

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC3 {
    pub(super) cart: Cartridge,
    ram_and_timer_enabled: bool,
    pub(super) rtc: Option<Rtc>,
}

impl CartridgeMBC3 {
    pub fn new(cart: Cartridge, has_rtc: bool) -> Self {
        Self {
            cart,
            ram_and_timer_enabled: false,
            rtc: if has_rtc { Some(Rtc::new()) } else { None },
        }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                let was_enabled = self.ram_and_timer_enabled;
                self.ram_and_timer_enabled = byte == 0x0A;
                if was_enabled
                    && !self.ram_and_timer_enabled
                    && let Err(e) = self.cart.save()
                {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                self.cart.rom_bank = if byte == 0 { 1 } else { byte.into() };
            }
            0x4000 | 0x5000 => match byte {
                0x0..=0x3 => {
                    self.cart.mode = 0;
                    self.cart.ram_bank = byte & 3;
                }
                0x8..=0xC => {
                    self.cart.mode = 1;
                    self.cart.ram_bank = byte;
                }
                _ => {}
            },
            0x6000 | 0x7000 => {
                if let Some(rtc) = &mut self.rtc {
                    match byte {
                        0x00 => rtc.latch_saw_zero = true,
                        0x01 if rtc.latch_saw_zero => {
                            rtc.latch();
                            rtc.latch_saw_zero = false;
                        }
                        _ => rtc.latch_saw_zero = false,
                    }
                }
            }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if self.cart.mode == 1 {
            if !self.ram_and_timer_enabled {
                return 0xFF;
            }
            return match &self.rtc {
                Some(rtc) => rtc.read(self.cart.ram_bank - 0x08),
                None => 0xFF,
            };
        }
        if self.cart.ram.is_empty() || !self.ram_and_timer_enabled {
            0xFF
        } else {
            let offset = self.cart.ram_bank as usize * crate::cartridge::RAM_BANK_SIZE;
            self.cart.ram[offset + addr as usize]
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.cart.mode == 1 {
            if self.ram_and_timer_enabled {
                let reg = self.cart.ram_bank - 0x08;
                if let Some(rtc) = &mut self.rtc {
                    rtc.write(reg, byte);
                }
            }
            return;
        }
        let ram_and_timer_enabled = self.ram_and_timer_enabled;
        let cartridge = &mut self.cart;
        if cartridge.ram.is_empty() || !ram_and_timer_enabled {
            return;
        }
        let offset = cartridge.ram_bank as usize * crate::cartridge::RAM_BANK_SIZE;
        cartridge.ram[offset + addr as usize] = byte;
        cartridge.ram_dirty = true;
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_decomposes_elapsed_correctly() {
        let elapsed: u64 = 86400 + 2 * 3600 + 3 * 60 + 4; // 1d 2h 3m 4s
        let mut rtc = Rtc {
            base_unix_secs: now_unix_secs() - elapsed,
            latched: [0; 5],
            latch_saw_zero: false,
        };
        rtc.latch();
        assert_eq!(rtc.latched[0], 4); // seconds
        assert_eq!(rtc.latched[1], 3); // minutes
        assert_eq!(rtc.latched[2], 2); // hours
        assert_eq!(rtc.latched[3], 1); // days_lo
        assert_eq!(rtc.latched[4], 0); // days_hi: no overflow
    }

    #[test]
    fn latch_sets_day256_bit() {
        let elapsed: u64 = 256 * 86400;
        let mut rtc = Rtc {
            base_unix_secs: now_unix_secs() - elapsed,
            latched: [0; 5],
            latch_saw_zero: false,
        };
        rtc.latch();
        assert_eq!(rtc.latched[3], 0); // days_lo = 256 & 0xFF = 0
        assert_eq!(rtc.latched[4] & 0x01, 0x01); // day-256 bit set
        assert_eq!(rtc.latched[4] & 0x80, 0x00); // no carry yet
    }

    #[test]
    fn latch_sets_carry_at_512_days() {
        let elapsed: u64 = 512 * 86400;
        let mut rtc = Rtc {
            base_unix_secs: now_unix_secs() - elapsed,
            latched: [0; 5],
            latch_saw_zero: false,
        };
        rtc.latch();
        assert_eq!(rtc.latched[4] & 0x80, 0x80); // carry bit
    }

    #[test]
    fn write_round_trips_all_registers() {
        let mut rtc = Rtc {
            base_unix_secs: now_unix_secs(),
            latched: [0; 5],
            latch_saw_zero: false,
        };
        rtc.write(0, 15); // seconds
        rtc.write(1, 30); // minutes
        rtc.write(2, 5); // hours
        rtc.write(3, 2); // days_lo
        rtc.write(4, 0); // days_hi
        rtc.latch();
        assert_eq!(rtc.latched[0], 15);
        assert_eq!(rtc.latched[1], 30);
        assert_eq!(rtc.latched[2], 5);
        assert_eq!(rtc.latched[3], 2);
        assert_eq!(rtc.latched[4], 0);
    }
}
