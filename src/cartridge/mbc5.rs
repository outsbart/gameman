use cartridge::Cartridge;

pub struct CartridgeMBC5 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8,
    rom_offset: usize,
    ram_offset: usize,
    mode: u8,
}

impl CartridgeMBC5 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {

        CartridgeMBC5 {
            rom, ram: vec![0u8; ram_size],
            ram_enabled: false,
            rom_bank: 1, rom_offset: 0x4000,
            ram_bank: 0, ram_offset: 0,
            mode: 0,
        }
    }
}


impl Cartridge for CartridgeMBC5 {
    fn read_rom(&mut self, addr: u16) -> u8 {
        let abs_addr = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => addr as usize,
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.rom_offset + (addr & 0x3FFF) as usize
            }
            _ => panic!("Unhandled ROM MBC5 read at addr {:x}", addr)
        };

        if abs_addr < self.rom.len() { self.rom[abs_addr] } else { 0 }
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {  // enable eram
                self.ram_enabled = byte == 0x0A;
            }
            0x2000 => {  // receive low bits of rom bank number
                self.rom_bank = (self.rom_bank & 0x100) | byte as u16;
                self.rom_offset = self.rom_bank as usize * 0x4000;
            },
            0x3000 => {  // receive high bit of rom bank number
                self.rom_bank = ((byte as u16 & 0x1) << 8) | (self.rom_bank & 0xFF);
                self.rom_offset = self.rom_bank as usize * 0x4000;
            }
            0x4000 | 0x5000 => {  // change ram bank
                self.ram_bank = byte & 0xF;
                self.ram_offset = self.ram_bank as usize * 0x2000;
            }
            0x6000 | 0x7000 => { }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr)
        };
    }

    fn read_ram(&mut self, addr: u16) -> u8 {
        if self.ram.is_empty() || !self.ram_enabled {
            0xFF
        } else {
            self.ram[self.ram_offset + addr as usize]
        }
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ram.is_empty() || !self.ram_enabled {
            return
        }
        self.ram[self.ram_offset + addr as usize] = byte;
    }
}
