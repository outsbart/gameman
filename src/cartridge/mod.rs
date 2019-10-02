pub mod nombc;
pub mod mbc1;
pub mod mbc3;
pub mod mbc5;

use std::fs::File;
use std::io::Read;

use cartridge::nombc::CartridgeNoMBC;
use cartridge::mbc1::CartridgeMBC1;
use cartridge::mbc5::CartridgeMBC5;
use cartridge::mbc3::CartridgeMBC3;


pub trait Cartridge {
    fn read_rom(&mut self, addr: u16) -> u8;
    fn write_rom(&mut self, addr: u16, byte: u8);
    fn read_ram(&mut self, addr: u16) -> u8;
    fn write_ram(&mut self, addr: u16, byte: u8);
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

    let ram_size = match rom[0x149] {
        0x00 => 0,
        0x01 => 2,
        0x02 => 8,
        0x03 => 32,
        0x04 => 128,
        0x05 => 64,
        _ => panic!("Unrecognized cartridge ram size")
    } * 1024;

    let cart_type = rom[0x147] as usize;

    println!("rom size = 0x{:x}", rom.len());
    println!("rom type = 0x{:x}", cart_type);
    println!("ram size = 0x{:x}", ram_size);

    match cart_type {
        0 => Box::new(CartridgeNoMBC::new(rom)),
        1|2|3 => Box::new(CartridgeMBC1::new(rom, ram_size)),
        0x13 => Box::new(CartridgeMBC3::new(rom, ram_size)),
        0x19|0x1b => Box::new(CartridgeMBC5::new(rom, ram_size)),
        _ => panic!("Cartridge type {:x} not implemented", cart_type)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_load_mbc1() {
        load_rom("tests/cpu_instrs/cpu_instrs.gb");
    }
}
