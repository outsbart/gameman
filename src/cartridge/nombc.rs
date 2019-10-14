use cartridge::{Cartridge, CartridgeAccess};

pub struct CartridgeNoMBC {
    cart: Cartridge
}

impl CartridgeNoMBC {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }
}

impl CartridgeAccess for CartridgeNoMBC {
    fn cartridge(&self) -> &Cartridge { &self.cart }
    fn cartridge_mut(&mut self) -> &mut Cartridge { &mut self.cart }
    fn read_rom(&self, addr: u16) -> u8 { self.cart.rom[addr as usize] }
    fn write_rom(&mut self, _addr: u16, _byte: u8) {}
    fn read_ram(&self, _addr: u16) -> u8 { 0xFF }
    fn write_ram(&mut self, _addr: u16, _byte: u8) {}
}
