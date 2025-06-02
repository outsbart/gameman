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
