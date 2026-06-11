use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC5 {
    pub(super) cart: Cartridge,
}

impl CartridgeMBC5 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = &mut self.cart;

        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram
                if let Err(e) = cartridge.update_ram_enabled(byte == 0x0A) {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x2000 => {
                // receive low bits of rom bank number
                cartridge.rom_bank = (cartridge.rom_bank & 0x100) | byte as u16;
            }
            0x3000 => {
                // receive high bit of rom bank number
                cartridge.rom_bank = ((byte as u16 & 0x1) << 8) | (cartridge.rom_bank & 0xFF);
            }
            0x4000 | 0x5000 => {
                // change ram bank
                cartridge.ram_bank = byte & 0xF;
            }
            0x6000 | 0x7000 => {}
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        self.cart.read_ram(addr)
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        self.cart.write_ram(addr, byte)
    }

}
