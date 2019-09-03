use std::fs::File;
use std::io::Read;


pub struct CartridgeNoMBC {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
}

pub struct CartridgeMBC1 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    rom_bank: u8,
    rom_offset: usize,
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
                if byte == 0x0A {
                    panic!("failed to enable eram");
                }
            }
            0x2000 | 0x3000 => {  // change rom bank
                let mut val:u8 = byte & 0x1F;
                if val == 0 { val = 1 };

                self.rom_bank = (self.rom_bank & 0x60) + val;
                self.rom_offset = self.rom_bank as usize * 0x4000;
            }
            0x4000 | 0x5000 => {  // change rom bank
                self.rom_bank = (self.rom_bank & 0x1F) + ((byte & 3) << 5);
                self.rom_offset = self.rom_bank as usize * 0x4000;
            }
            0x6000 | 0x7000 => {} // change rom mode
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr)
        };
    }

    fn read_ram(&mut self, addr: u16) -> u8 {
        if self.ram.is_empty() {
            return 0xFF;
        }
        panic!("ERAM read not implemented yet!")
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ram.is_empty() {
            return
        }
        panic!("ERAM write not implemented yet!")
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

    let ram_size = rom[0x149] as usize;
    let cart_type = rom[0x147] as usize;

    println!("rom capacity = {}kb, len = {:x}", rom.capacity()/1024, rom.len());
    println!("rom type  = {}", cart_type);
    println!("ram size = {}", ram_size);

    let ram = vec![0u8; ram_size];

    match cart_type {
        0 => Box::new(CartridgeNoMBC { rom, ram }),
        1|2|3 => Box::new(CartridgeMBC1 { rom, ram, rom_bank: 0, rom_offset: 0 }),
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
