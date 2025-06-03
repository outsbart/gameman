use crate::cartridge::{Cartridge, RAM_BANK_SIZE, ROM_BANK_SIZE};
use serde::{Deserialize, Serialize};
use std::io;

/// HuC1 (cart type 0xFF) — MBC1-compatible banking with an IR LED/receiver.
/// The only behavioral difference from MBC1 is the 0x0000-0x1FFF register:
/// writing 0x0A enables normal RAM access; anything else enables IR mode,
/// which we stub (reads return 0xFF, writes are ignored).
#[derive(Serialize, Deserialize)]
pub struct CartridgeHuC1 {
    pub(super) cart: Cartridge,
    ir_mode: bool,
}

impl CartridgeHuC1 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ir_mode: true,
        }
    }

    fn ram_offset(&self) -> usize {
        let cart = &self.cart;
        if cart.mode == 0 || cart.ram.is_empty() {
            return 0;
        }
        let num_ram_banks = cart.ram.len() / RAM_BANK_SIZE;
        (cart.ram_bank as usize & (num_ram_banks - 1)) * RAM_BANK_SIZE
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let cart = &self.cart;
        let num_banks = cart.rom.len() / ROM_BANK_SIZE;
        let bank = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => {
                if cart.mode == 1 {
                    (cart.rom_bank & 0x60) as usize & (num_banks - 1)
                } else {
                    0
                }
            }
            0x4000 | 0x5000 | 0x6000 | 0x7000 => cart.rom_bank as usize & (num_banks - 1),
            _ => panic!("Unhandled HuC1 ROM read at addr {:x}", addr),
        };
        cart.rom[bank * ROM_BANK_SIZE + (addr & 0x3FFF) as usize]
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                let ram_enable = byte == 0x0A;
                self.ir_mode = !ram_enable;
                if let Err(e) = self.cart.update_ram_enabled(ram_enable) {
                    println!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                let val = (byte & 0x1F).max(1);
                self.cart.rom_bank = (self.cart.rom_bank & 0x60) | val as u16;
            }
            0x4000 | 0x5000 => {
                self.cart.rom_bank = (self.cart.rom_bank & 0x1F) | ((byte as u16 & 0x03) << 5);
                if self.cart.mode == 1 {
                    self.cart.ram_bank = byte & 0x03;
                }
            }
            0x6000 | 0x7000 => {
                self.cart.mode = byte & 0x01;
            }
            _ => panic!("Unhandled HuC1 ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if self.ir_mode {
            return 0xFF;
        }
        let cart = &self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return 0xFF;
        }
        cart.ram[self.ram_offset() + addr as usize]
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ir_mode {
            return;
        }
        let offset = self.ram_offset();
        let cart = &mut self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return;
        }
        cart.ram[offset + addr as usize] = byte;
        cart.ram_dirty = true;
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, RAM_BANK_SIZE, ROM_BANK_SIZE};
    use std::path::PathBuf;

    fn make_huc1(num_rom_banks: usize) -> CartridgeHuC1 {
        let mut rom = vec![0u8; num_rom_banks * ROM_BANK_SIZE];
        for b in 0..num_rom_banks {
            rom[b * ROM_BANK_SIZE + 0x100] = b as u8;
        }
        let cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        CartridgeHuC1::new(cart)
    }

    #[test]
    fn ir_mode_on_by_default() {
        let mut c = make_huc1(2);
        c.cart.ram = vec![0x42; RAM_BANK_SIZE];
        assert_eq!(c.read_ram(0), 0xFF);
    }

    #[test]
    fn write_0a_disables_ir_mode() {
        let mut c = make_huc1(2);
        c.cart.ram = vec![0u8; RAM_BANK_SIZE];
        c.cart.ram[0] = 0x42;
        c.write_rom(0x0000, 0x0A);
        assert_eq!(c.read_ram(0), 0x42);
    }

    #[test]
    fn non_0a_write_reenables_ir_mode() {
        let mut c = make_huc1(2);
        c.cart.ram = vec![0u8; RAM_BANK_SIZE];
        c.cart.ram[0] = 0x42;
        c.write_rom(0x0000, 0x0A); // enable RAM
        c.write_rom(0x0000, 0x00); // back to IR mode
        c.write_ram(0, 0xFF); // ignored in IR mode
        c.write_rom(0x0000, 0x0A); // re-enable to verify
        assert_eq!(c.read_ram(0), 0x42);
    }

    #[test]
    fn rom_bank_zero_clamps_to_one() {
        let mut c = make_huc1(2);
        c.write_rom(0x2000, 0x00);
        assert_eq!(c.read_rom(0x4100), 1);
    }

    #[test]
    fn upper_rom_bank_bits_from_0x4000() {
        let mut c = make_huc1(64);
        c.write_rom(0x2000, 0x01); // low bits = 1
        c.write_rom(0x4000, 0x01); // upper bits = 0b01 → rom_bank = 0x21 = 33
        assert_eq!(c.read_rom(0x4100), 33);
    }

    #[test]
    fn mode1_redirects_bank0_window() {
        let mut c = make_huc1(64);
        c.write_rom(0x2000, 0x01); // low bits = 1
        c.write_rom(0x4000, 0x01); // rom_bank = 0x21; upper bits in 0x60 mask = 0x20 = 32
        assert_eq!(c.read_rom(0x0100), 0); // mode 0: bank-0 window = bank 0
        c.write_rom(0x6000, 0x01); // switch to mode 1
        assert_eq!(c.read_rom(0x0100), 32); // (0x21 & 0x60) & 63 = 32
    }
}
