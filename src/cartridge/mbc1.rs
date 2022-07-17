use cartridge::{Cartridge, CartridgeAccess};

pub struct CartridgeMBC1 {
    cart: Cartridge,
}

impl CartridgeMBC1 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }
}

impl CartridgeAccess for CartridgeMBC1 {
    fn cartridge(&self) -> &Cartridge {
        &self.cart
    }
    fn cartridge_mut(&mut self) -> &mut Cartridge {
        &mut self.cart
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = self.cartridge_mut();

        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram
                cartridge.ram_enabled = byte == 0x0A;
            }
            0x2000 | 0x3000 => {
                // change rom bank
                let mut val: u8 = byte & 0x1F;
                if val == 0 {
                    val = 1
                };

                cartridge.rom_bank = (cartridge.rom_bank & 0x60) + val as u16;
            }
            0x4000 | 0x5000 => {
                // change rom bank or ram bank
                if cartridge.mode == 1 {
                    cartridge.ram_bank = byte & 3;
                } else {
                    cartridge.rom_bank = (cartridge.rom_bank & 0x1F) + ((byte & 3) << 5) as u16;
                }
            }
            0x6000 | 0x7000 => {
                panic!("rom mode change not implemented")
            } // change rom mode
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }
}
