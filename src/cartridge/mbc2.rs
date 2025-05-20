use crate::cartridge::{Cartridge, CartridgeAccess, ROM_BANK_SIZE};

pub struct CartridgeMBC2 {
    cart: Cartridge,
}

impl CartridgeMBC2 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }
}

impl CartridgeAccess for CartridgeMBC2 {
    fn cartridge(&self) -> &Cartridge {
        &self.cart
    }
    fn cartridge_mut(&mut self) -> &mut Cartridge {
        &mut self.cart
    }

    fn read_rom(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge();

        let abs_addr = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => addr as usize,
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                let num_banks = cartridge.rom.len() / ROM_BANK_SIZE;
                let bank = cartridge.rom_bank as usize & (num_banks - 1);
                bank * ROM_BANK_SIZE + (addr & 0x3FFF) as usize
            }
            _ => panic!("Unhandled ROM MBC2 read at addr {:x}", addr),
        };

        cartridge.rom[abs_addr]
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        if addr > 0x3FFF {
            return;
        }

        let cartridge = self.cartridge_mut();

        if addr & 0x0100 == 0 {
            cartridge.ram_enabled = byte & 0x0F == 0x0A;
        } else {
            let mut bank = byte & 0x0F;
            if bank == 0 {
                bank = 1;
            }
            cartridge.rom_bank = bank as u16;
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge();

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return 0xFF;
        }

        cartridge.ram[(addr & 0x01FF) as usize] | 0xF0
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        let cartridge = self.cartridge_mut();

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return;
        }

        cartridge.ram[(addr & 0x01FF) as usize] = byte & 0x0F;
    }
}
