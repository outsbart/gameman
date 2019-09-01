use std::fs::File;
use std::io::Read;



pub struct Cartridge {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
}

impl Cartridge {
    pub fn from_rom(path: &str) -> Cartridge {
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

        println!("rom capacity = {}kb, len = {:x}", rom.capacity()/1024, rom.len());
        println!("ram size = {}", ram_size);

        Cartridge {
            rom,
            ram: vec![0; ram_size],
        }

    }

    pub fn read_rom(&mut self, addr: u16) -> u8 { self.rom[addr as usize] }

    pub fn read_ram(&mut self, addr: u16) -> u8 {
        if self.ram.is_empty() {
            return 0xFF;
        }
        self.ram[addr as usize]
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {

    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ram.is_empty() {
            return
        }
        self.ram[addr as usize] = byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_load() {
        Cartridge::from_rom("tests/cpu_instrs/cpu_instrs.gb");
    }
}
