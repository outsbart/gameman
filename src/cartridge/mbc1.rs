use cartridge::Cartridge;

pub struct CartridgeMBC1 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    ram_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    rom_offset: usize,
    ram_offset: usize,
    mode: u8,
}

impl CartridgeMBC1 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        CartridgeMBC1 {
            rom, ram: vec![0; ram_size],
            ram_enabled: false,
            rom_bank: 1, rom_offset: 0x4000,
            ram_bank: 0, ram_offset: 0,
            mode: 0,
        }
    }
}

impl Cartridge for CartridgeMBC1 {
    fn read_rom(&mut self, addr: u16) -> u8 {
        let abs_addr = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => addr as usize,
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.rom_offset + (addr & 0x3FFF) as usize
            }
            _ => panic!("Unhandled ROM MBC1 read at addr {:x}", addr)
        };

        if abs_addr < self.rom.len() { self.rom[abs_addr] } else { 0 }
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {  // enable eram
                self.ram_enabled = byte == 0x0A;
            }
            0x2000 | 0x3000 => {  // change rom bank
                let mut val:u8 = byte & 0x1F;
                if val == 0 { val = 1 };

                self.rom_bank = (self.rom_bank & 0x60) + val;
                self.rom_offset = self.rom_bank as usize * 0x4000;
            }
            0x4000 | 0x5000 => {  // change rom bank or ram bank
                if self.mode == 1 {
                    self.ram_bank = byte & 3;
                    self.ram_offset = self.ram_bank as usize * 0x2000;
                } else {
                    self.rom_bank = (self.rom_bank & 0x1F) + ((byte & 3) << 5);
                    self.rom_offset = self.rom_bank as usize * 0x4000;
                }
            }
            0x6000 | 0x7000 => { panic!("rom mode change not implemented") } // change rom mode
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
