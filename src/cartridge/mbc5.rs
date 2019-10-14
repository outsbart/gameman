use cartridge::{Cartridge, CartridgeAccess};

pub struct CartridgeMBC5 {
    cart: Cartridge
}

impl CartridgeMBC5 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }
}


impl CartridgeAccess for CartridgeMBC5 {
    fn cartridge(&self) -> &Cartridge { &self.cart }
    fn cartridge_mut(&mut self) -> &mut Cartridge { &mut self.cart }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = self.cartridge_mut();

        match addr & 0xF000 {
            0x0000 | 0x1000 => {  // enable eram
                cartridge.ram_enabled = byte == 0x0A;
            }
            0x2000 => {  // receive low bits of rom bank number
                cartridge.rom_bank = (cartridge.rom_bank & 0x100) | byte as u16;
            },
            0x3000 => {  // receive high bit of rom bank number
                cartridge.rom_bank = ((byte as u16 & 0x1) << 8) | (cartridge.rom_bank & 0xFF);
            }
            0x4000 | 0x5000 => {  // change ram bank
                cartridge.ram_bank = byte & 0xF;
            }
            0x6000 | 0x7000 => { }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr)
        };
    }
}
