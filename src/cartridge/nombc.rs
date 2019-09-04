use cartridge::Cartridge;

pub struct CartridgeNoMBC {
    pub rom: Vec<u8>,
}


impl Cartridge for CartridgeNoMBC {
    fn read_rom(&mut self, addr: u16) -> u8 { self.rom[addr as usize] }
    fn write_rom(&mut self, _addr: u16, _byte: u8) {}
    fn read_ram(&mut self, _addr: u16) -> u8 { 0xFF }
    fn write_ram(&mut self, _addr: u16, _byte: u8) {}
}

impl CartridgeNoMBC {
    pub fn new(rom: Vec<u8>) -> Self {
        CartridgeNoMBC { rom }
    }
}
