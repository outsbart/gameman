use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize)]
pub struct CartridgeNoMBC {
    pub(super) cart: Cartridge,
}

impl CartridgeNoMBC {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.rom[addr as usize]
    }
    pub fn write_rom(&mut self, _addr: u16, _byte: u8) {}
    pub fn read_ram(&self, _addr: u16) -> u8 {
        0xFF
    }
    pub fn write_ram(&mut self, _addr: u16, _byte: u8) {}
    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}
