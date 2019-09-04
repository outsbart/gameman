use std::fs::File;
use std::io::Read;


pub struct CartridgeNoMBC {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
}

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
    pub fn new(rom: Vec<u8>, ram: Vec<u8>) -> Self {
        CartridgeMBC1 {
            rom, ram,
            ram_enabled: false,
            rom_bank: 0, rom_offset: 0,
            ram_bank: 0, ram_offset: 0,
            mode: 0,
        }
    }
}

pub trait Cartridge {
    fn read_rom(&mut self, addr: u16) -> u8;
    fn write_rom(&mut self, addr: u16, byte: u8);
    fn read_ram(&mut self, addr: u16) -> u8;
    fn write_ram(&mut self, addr: u16, byte: u8);
}


impl Cartridge for CartridgeNoMBC {
    fn read_rom(&mut self, addr: u16) -> u8 { self.rom[addr as usize] }
    fn write_rom(&mut self, _addr: u16, _byte: u8) {}

    fn read_ram(&mut self, addr: u16) -> u8 {
        if self.ram.is_empty() {
            return 0xFF;
        }
        self.ram[addr as usize]
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ram.is_empty() {
            return
        }
        self.ram[addr as usize] = byte
    }
}


impl Cartridge for CartridgeMBC1 {
    fn read_rom(&mut self, addr: u16) -> u8 {
        match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => self.rom[addr as usize],
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.rom[self.rom_offset + (addr & 0x3FFF) as usize]
            }
            _ => panic!("Unhandled ROM MBC1 read at addr {:x}", addr)
        }
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


pub fn load_rom(path: &str) -> Box<Cartridge> {
    let mut rom: Vec<u8> = Vec::new();

    match File::open(path) {
        Ok(mut file) => {
            match file.read_to_end(&mut rom) {
                Ok(_) => {},
                Err(_) => panic!("couldnt read the rom into the buffer!"),
            };
        }
        Err(_) => panic!("couldnt open the rom file"),
    }

    let ram_size = ((32 * 1024) << rom[0x149]) as usize;
    let cart_type = rom[0x147] as usize;

    println!("rom capacity = {}kb, len = {:x}", rom.capacity()/1024, rom.len());
    println!("rom type  = {}", cart_type);
    println!("ram size = {:x}", ram_size);

    let ram = vec![0u8; ram_size];

    match cart_type {
        0 => Box::new(CartridgeNoMBC { rom, ram }),
        1|2|3 => Box::new(CartridgeMBC1::new(rom, ram)),
        _ => panic!("Cartridge type {} not implemented", cart_type)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_load() {
        load_rom("tests/cpu_instrs/cpu_instrs.gb");
    }
}
