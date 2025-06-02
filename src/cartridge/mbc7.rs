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
