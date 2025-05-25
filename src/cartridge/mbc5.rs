use crate::cartridge::{Cartridge, CartridgeAccess, ROM_BANK_SIZE};

pub struct CartridgeMBC5 {
    cart: Cartridge,
}

impl CartridgeMBC5 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }
}

impl CartridgeAccess for CartridgeMBC5 {
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
            _ => panic!("Unhandled ROM MBC5 read at addr {:x}", addr),
        };

        cartridge.rom[abs_addr]
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = self.cartridge_mut();
        let mut should_save = false;

        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram
                let was_enabled = cartridge.ram_enabled;
                cartridge.ram_enabled = byte == 0x0A;
                should_save = was_enabled && !cartridge.ram_enabled;
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
        if should_save {
            if let Err(e) = self.cart.save() {
                println!("Error saving: {}", e);
            }
        }
    }
}
