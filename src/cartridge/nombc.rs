use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize)]
pub struct CartridgeNoMBC {
    pub(super) cart: Cartridge,
}

impl CartridgeNoMBC {
    pub fn new(cart: Cartridge) -> Self {
        let mut c = Self { cart };
        if !c.cart.ram.is_empty() {
            let _ = c.cart.update_ram_enabled(true);
        }
        c
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.rom[addr as usize]
    }
    pub fn write_rom(&mut self, _addr: u16, _byte: u8) {}
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.cart.read_ram(addr)
    }
    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        self.cart.write_ram(addr, byte)
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

    #[test]
    fn ram_auto_enabled_on_construction() {
        let rom = vec![0u8; ROM_BANK_SIZE];
        let mut cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        cart.ram = vec![0u8; RAM_BANK_SIZE];
        cart.ram[0] = 0x42;
        let mut c = CartridgeNoMBC::new(cart);
        // No write_rom call needed — RAM is accessible immediately
        assert_eq!(c.read_ram(0), 0x42);
        c.write_ram(5, 0xBB);
        assert_eq!(c.read_ram(5), 0xBB);
    }

    #[test]
    fn write_rom_is_a_no_op() {
        let rom = vec![0u8; ROM_BANK_SIZE];
        let cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        let mut c = CartridgeNoMBC::new(cart);
        c.write_rom(0x0000, 0xFF); // shouldn't panic or change anything
        c.write_rom(0x7FFF, 0xFF);
        assert_eq!(c.read_rom(0), 0); // ROM unchanged
    }
}
