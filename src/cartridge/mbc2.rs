use crate::cartridge::Cartridge;
use std::io;

pub struct CartridgeMBC2 {
    cart: Cartridge,
}

impl CartridgeMBC2 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        if addr > 0x3FFF {
            return;
        }

        let cartridge = &mut self.cart;

        if addr & 0x0100 == 0 {
            if let Err(e) = cartridge.update_ram_enabled(byte & 0x0F == 0x0A) {
                println!("Error saving: {}", e);
            }
        } else {
            let mut bank = byte & 0x0F;
            if bank == 0 {
                bank = 1;
            }
            cartridge.rom_bank = bank as u16;
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = &self.cart;

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return 0xFF;
        }

        cartridge.ram[(addr & 0x01FF) as usize] | 0xF0
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        let cartridge = &mut self.cart;

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return;
        }

        cartridge.ram[(addr & 0x01FF) as usize] = byte & 0x0F;
        cartridge.ram_dirty = true;
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}
