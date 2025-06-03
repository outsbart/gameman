use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::io;

/// MBC7 (cart type 0x22) — ROM banking + 93C56 EEPROM (128×16-bit) + accelerometer stub.
///
/// ERAM window (0xA000-0xBFFF) is a register file, not plain RAM:
///   0xA000: accelerometer latch (write triggers latch; read: 0x00=latched, 0xFF=not)
///   0xA002-0xA003: latched X axis (16-bit LE, 0x8000 = center)
///   0xA004-0xA005: latched Y axis (16-bit LE, 0x8000 = center)
///   0xA006-0xA007: mirror of high bytes of X and Y
///   0xA080: EEPROM serial register (CS/CLK/DI/DO bit-bang)
///
/// EEPROM uses cart.ram[0..256] as backing storage.
/// RAM enable requires two writes: 0x0A to 0x0000-0x1FFF then 0x40 to 0x4000-0x5FFF.
#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC7 {
    pub(super) cart: Cartridge,
    ram_enabled_1: bool,
    ram_enabled_2: bool,
    accel_latched: bool,
    accel_x: u16,
    accel_y: u16,
    // 93C56 serial interface state
    cs: bool,
    clk: bool,
    do_out: bool,
    write_enabled: bool,
    // state: 0=idle, 1=receiving_command, 2=reading, 3=writing
    phase: u8,
    shift_reg: u16,
    shift_cnt: u8,
    out_reg: u16,
    out_cnt: u8,
    in_reg: u16,
    in_cnt: u8,
    eeprom_addr: u8,
}

impl CartridgeMBC7 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ram_enabled_1: false,
            ram_enabled_2: false,
            accel_latched: false,
            accel_x: 0x8000,
            accel_y: 0x8000,
            cs: false,
            clk: false,
            do_out: true,
            write_enabled: false,
            phase: 0,
            shift_reg: 0,
            shift_cnt: 0,
            out_reg: 0,
            out_cnt: 0,
            in_reg: 0,
            in_cnt: 0,
            eeprom_addr: 0,
        }
    }

    fn ram_enabled(&self) -> bool {
        self.ram_enabled_1 && self.ram_enabled_2
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                self.ram_enabled_1 = byte == 0x0A;
                if !self.ram_enabled_1 {
                    self.ram_enabled_2 = false;
                }
            }
            0x2000 | 0x3000 => {
                self.cart.rom_bank = ((byte & 0x7F) as u16).max(1);
            }
            0x4000 | 0x5000 => {
                if byte == 0x40 {
                    self.ram_enabled_2 = self.ram_enabled_1;
                } else {
                    self.ram_enabled_1 = false;
                    self.ram_enabled_2 = false;
                }
            }
            0x6000 | 0x7000 => {}
            _ => panic!("Unhandled MBC7 ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if !self.ram_enabled() {
            return 0xFF;
        }
        match addr & 0x00FF {
            0x00 if self.accel_latched => 0x00,
            0x02 if self.accel_latched => (self.accel_x & 0xFF) as u8,
            0x03 if self.accel_latched => (self.accel_x >> 8) as u8,
            0x04 if self.accel_latched => (self.accel_y & 0xFF) as u8,
            0x05 if self.accel_latched => (self.accel_y >> 8) as u8,
            0x06 if self.accel_latched => (self.accel_x >> 8) as u8,
            0x07 if self.accel_latched => (self.accel_y >> 8) as u8,
            0x80 => (self.do_out as u8) << 1,
            _ => 0xFF,
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        match addr & 0x00FF {
            0x00 => {
                self.accel_latched = true;
                // Stub: axes remain at 0x8000 (center / no tilt)
            }
            0x80 => {
                self.eeprom_write(byte);
            }
            _ => {}
        }
    }

    fn eeprom_write(&mut self, byte: u8) {
        let cs = byte & 0x80 != 0;
        let clk = byte & 0x40 != 0;
        let di = byte & 0x02 != 0;

        // CS falling edge → reset to idle
        if self.cs && !cs {
            self.phase = 0;
        }

        // CLK rising edge while CS high
        if cs && clk && !self.clk {
            self.eeprom_clk(di);
        }

        self.cs = cs;
        self.clk = clk;
    }

    fn eeprom_clk(&mut self, di: bool) {
        match self.phase {
            0
                // Idle: first DI=1 is the start bit
                if di => {
                    self.shift_reg = 1;
                    self.shift_cnt = 1;
                    self.phase = 1;
                }
            1 => {
                // Receiving command: shift in until we have 10 bits
                // Layout: [9]=start [8:7]=opcode [6:0]=addr (7-bit for 128-word EEPROM)
                self.shift_reg = (self.shift_reg << 1) | di as u16;
                self.shift_cnt += 1;
                if self.shift_cnt == 10 {
                    self.decode_command();
                }
            }
            2 => {
                // Reading: clock out data MSB-first
                if self.out_cnt > 0 {
                    self.out_cnt -= 1;
                    self.do_out = (self.out_reg >> self.out_cnt) & 1 != 0;
                } else {
                    self.phase = 0;
                }
            }
            3 => {
                // Writing: clock in 16 data bits
                self.in_reg = (self.in_reg << 1) | di as u16;
                self.in_cnt += 1;
                if self.in_cnt == 16 {
                    self.finish_write();
                    self.do_out = true; // signal ready
                    self.phase = 0;
                }
            }
            _ => {}
        }
    }

    fn decode_command(&mut self) {
        let opcode = (self.shift_reg >> 7) & 0x03;
        let addr = (self.shift_reg & 0x7F) as u8;
        self.eeprom_addr = addr;

        match opcode {
            0b10 => {
                // READ: load word and clock out
                let lo = self
                    .cart
                    .ram
                    .get(addr as usize * 2)
                    .copied()
                    .unwrap_or(0xFF);
                let hi = self
                    .cart
                    .ram
                    .get(addr as usize * 2 + 1)
                    .copied()
                    .unwrap_or(0xFF);
                self.out_reg = (hi as u16) << 8 | lo as u16;
                self.out_cnt = 16;
                self.do_out = false; // DO low before first data bit
                self.phase = 2;
            }
            0b01 => {
                // WRITE: receive 16 data bits
                self.in_reg = 0;
                self.in_cnt = 0;
                self.phase = 3;
            }
            0b11 => {
                // ERASE: fill word with 0xFFFF
                if self.write_enabled {
                    let a = addr as usize;
                    if a * 2 + 1 < self.cart.ram.len() {
                        self.cart.ram[a * 2] = 0xFF;
                        self.cart.ram[a * 2 + 1] = 0xFF;
                        self.cart.ram_dirty = true;
                    }
                }
                self.do_out = true;
                self.phase = 0;
            }
            0b00 => {
                // Extended commands (opcode=00, subcommand in addr bits 6:5)
                let subcmd = (addr >> 5) & 0x03;
                match subcmd {
                    0b11 => self.write_enabled = true,  // EWEN
                    0b00 => self.write_enabled = false, // EWDS
                    0b10
                        // ERAL: erase all words to 0xFFFF
                        if self.write_enabled => {
                            for b in self.cart.ram.iter_mut() {
                                *b = 0xFF;
                            }
                            self.cart.ram_dirty = true;
                        }
                    _ => {} // WRAL (write all): uncommon, stub as no-op
                }
                self.do_out = true;
                self.phase = 0;
            }
            _ => unreachable!(),
        }
    }

    fn finish_write(&mut self) {
        if self.write_enabled {
            let a = self.eeprom_addr as usize;
            if a * 2 + 1 < self.cart.ram.len() {
                self.cart.ram[a * 2] = (self.in_reg & 0xFF) as u8;
                self.cart.ram[a * 2 + 1] = (self.in_reg >> 8) as u8;
                self.cart.ram_dirty = true;
            }
        }
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, ROM_BANK_SIZE};
    use std::path::PathBuf;

    fn make_mbc7() -> CartridgeMBC7 {
        let rom = vec![0u8; 2 * ROM_BANK_SIZE];
        let mut cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        cart.ram = vec![0u8; 256]; // 128 × 16-bit EEPROM words
        CartridgeMBC7::new(cart)
    }

    fn enable_ram(c: &mut CartridgeMBC7) {
        c.write_rom(0x0000, 0x0A); // step 1
        c.write_rom(0x4000, 0x40); // step 2
    }

    // Toggle CLK to produce one rising edge with the given DI value.
    fn send_bit(c: &mut CartridgeMBC7, di: bool) {
        let b = if di { 0x02 } else { 0x00 };
        c.write_ram(0x0080, 0x80 | b); // CS=1, CLK=0
        c.write_ram(0x0080, 0x80 | 0x40 | b); // CS=1, CLK=1 (rising edge)
    }

    // Send start bit + 2-bit opcode + 7-bit address (10 bits total).
    fn eeprom_cmd(c: &mut CartridgeMBC7, opcode: u8, addr: u8) {
        c.write_ram(0x0080, 0x00); // CS=0 → reset EEPROM phase
        c.write_ram(0x0080, 0x80); // CS=1, CLK=0
        send_bit(c, true); // start bit
        send_bit(c, (opcode >> 1) & 1 != 0);
        send_bit(c, opcode & 1 != 0);
        for i in (0..7).rev() {
            send_bit(c, (addr >> i) & 1 != 0);
        }
    }

    // Clock in 16 data bits MSB-first (used after a WRITE command).
    fn eeprom_write_word(c: &mut CartridgeMBC7, word: u16) {
        for i in (0..16).rev() {
            send_bit(c, (word >> i) & 1 != 0);
        }
    }

    // Clock out 16 data bits MSB-first (used after a READ command).
    fn eeprom_read_word(c: &mut CartridgeMBC7) -> u16 {
        let mut word = 0u16;
        for _ in 0..16 {
            c.write_ram(0x0080, 0x80); // CS=1, CLK=0
            c.write_ram(0x0080, 0x80 | 0x40); // CS=1, CLK=1 → updates do_out
            word = (word << 1) | ((c.read_ram(0x0080) >> 1) & 1) as u16;
        }
        word
    }

    #[test]
    fn ram_disabled_reads_return_0xff() {
        let mut c = make_mbc7();
        c.write_rom(0x0000, 0x0A); // step 1 only, no step 2
        assert_eq!(c.read_ram(0x0080), 0xFF);
    }

    #[test]
    fn dual_enable_required() {
        let mut c = make_mbc7();
        c.write_rom(0x0000, 0x0A); // step 1
        assert_eq!(c.read_ram(0x0080), 0xFF); // not enabled yet
        c.write_rom(0x4000, 0x40); // step 2
        assert_eq!(c.read_ram(0x0080), 0x02); // do_out starts true → bit 1 set
    }

    #[test]
    fn wrong_second_byte_disables_ram() {
        let mut c = make_mbc7();
        c.write_rom(0x0000, 0x0A);
        c.write_rom(0x4000, 0x41); // wrong byte → both enables cleared
        assert_eq!(c.read_ram(0x0080), 0xFF);
    }

    #[test]
    fn accel_not_latched_reads_0xff() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        assert_eq!(c.read_ram(0x0002), 0xFF); // X axis lo — not yet latched
    }

    #[test]
    fn accel_latch_returns_center() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        c.write_ram(0x0000, 0x00); // latch accelerometer
        assert_eq!(c.read_ram(0x0002), 0x00); // X lo = 0x8000 & 0xFF
        assert_eq!(c.read_ram(0x0003), 0x80); // X hi = 0x8000 >> 8
        assert_eq!(c.read_ram(0x0004), 0x00); // Y lo
        assert_eq!(c.read_ram(0x0005), 0x80); // Y hi
    }

    #[test]
    fn eeprom_write_blocked_without_ewen() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        eeprom_cmd(&mut c, 0b01, 5); // WRITE addr 5, no EWEN
        eeprom_write_word(&mut c, 0xABCD);
        eeprom_cmd(&mut c, 0b10, 5); // READ addr 5
        assert_eq!(eeprom_read_word(&mut c), 0x0000); // backing RAM unchanged
    }

    #[test]
    fn eeprom_write_read_roundtrip() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        eeprom_cmd(&mut c, 0b00, 0x60); // EWEN (subcmd = 0b11)
        eeprom_cmd(&mut c, 0b01, 5); // WRITE addr 5
        eeprom_write_word(&mut c, 0xABCD);
        eeprom_cmd(&mut c, 0b10, 5); // READ addr 5
        assert_eq!(eeprom_read_word(&mut c), 0xABCD);
    }

    #[test]
    fn eeprom_erase_fills_word_with_0xffff() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        eeprom_cmd(&mut c, 0b00, 0x60); // EWEN
        eeprom_cmd(&mut c, 0b01, 3); // WRITE addr 3
        eeprom_write_word(&mut c, 0x1234);
        eeprom_cmd(&mut c, 0b11, 3); // ERASE addr 3
        eeprom_cmd(&mut c, 0b10, 3); // READ addr 3
        assert_eq!(eeprom_read_word(&mut c), 0xFFFF);
    }

    #[test]
    fn eeprom_eral_erases_all() {
        let mut c = make_mbc7();
        enable_ram(&mut c);
        eeprom_cmd(&mut c, 0b00, 0x60); // EWEN
        eeprom_cmd(&mut c, 0b01, 0); // WRITE addr 0
        eeprom_write_word(&mut c, 0x1111);
        eeprom_cmd(&mut c, 0b01, 1); // WRITE addr 1
        eeprom_write_word(&mut c, 0x2222);
        eeprom_cmd(&mut c, 0b00, 0x40); // ERAL (subcmd = 0b10)
        eeprom_cmd(&mut c, 0b10, 0); // READ addr 0
        assert_eq!(eeprom_read_word(&mut c), 0xFFFF);
        eeprom_cmd(&mut c, 0b10, 1); // READ addr 1
        assert_eq!(eeprom_read_word(&mut c), 0xFFFF);
    }
}
