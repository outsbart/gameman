pub mod nombc;
pub mod mbc1;
pub mod mbc3;
pub mod mbc5;

use cartridge::nombc::CartridgeNoMBC;
use cartridge::mbc1::CartridgeMBC1;
use cartridge::mbc5::CartridgeMBC5;
use cartridge::mbc3::CartridgeMBC3;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write, SeekFrom, Seek};
use std::path::PathBuf;
use std::io;

pub const ROM_BANK_SIZE: usize = 0x4000;
pub const RAM_BANK_SIZE: usize = 0x2000;

pub struct Cartridge {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    ram_size: usize,
    ram_enabled: bool,
    rom_bank: u16,
    ram_bank: u8,
    mode: u8,

    path: PathBuf,
    save_file: Option<File>,
}


impl Cartridge {
    pub fn new(path: PathBuf, rom: Vec<u8>, ram_size: usize) -> Self {
        let mut cart = Self {
            rom, ram: Vec::new(),
            ram_size,
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            mode: 0,
            path,
            save_file: None,
        };

        if ram_size > 0 {
            match cart.try_load_save_file() {
                Ok(file) => { cart.save_file = Some(file) },
                Err(e) => { println!("Unable to load/create save file: {}", e) }
            }
        }

        cart
    }

    // the path for the save file
    fn save_file_path(&self) -> PathBuf {
        let mut save_file = self.path.clone();
        save_file.set_extension("sav");
        save_file
    }

    // attemps to load/create a save file
    fn try_load_save_file(&mut self) -> io::Result<File> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.save_file_path())?;

        let file_size = file.metadata()?.len();
        let expected_file_size = self.ram_size as u64;

        if file_size == 0 {
            println!("Save file not found, creating one");
            self.ram = vec![0; self.ram_size];
            self.save()?
        } else if file_size != expected_file_size {
            panic!("Save file has unexpected size");
        } else {
            println!("Loading save file");
            file.read_to_end(&mut self.ram)?;
        };

        Ok(file)
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(file) = self.save_file.as_mut() {
            println!("Saving game");
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&self.ram)?;
        }
        Ok(())
    }
}

impl Drop for Cartridge {
    fn drop(&mut self) {
        // TODO: dont save when closing
        match self.save() {
            Ok(()) => {},
            Err(e) => { println!("Error updating save file: {}", e) }
        };
    }
}

pub trait CartridgeAccess {
    fn cartridge(&self) -> &Cartridge;
    fn cartridge_mut(&mut self) -> &mut Cartridge;

    fn ram_offset(&self) -> usize {
        let cartridge = self.cartridge();
        cartridge.ram_bank as usize * RAM_BANK_SIZE
    }
    fn rom_offset(&self) -> usize {
        let cartridge = self.cartridge();
        cartridge.rom_bank as usize * ROM_BANK_SIZE
    }

    fn read_rom(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge();

        let abs_addr = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => addr as usize,
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.rom_offset() + (addr & 0x3FFF) as usize
            }
            _ => panic!("Unhandled ROM MBC read at addr {:x}", addr)
        };

        if abs_addr < cartridge.rom.len() { cartridge.rom[abs_addr] } else { 0 }
    }

    fn write_rom(&mut self, addr: u16, byte: u8);

    fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge();

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            0xFF
        } else {
            cartridge.ram[self.ram_offset() + addr as usize]
        }
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        let ram_offset = self.ram_offset();

        let cartridge = self.cartridge_mut();

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return
        }
        cartridge.ram[ram_offset + addr as usize] = byte;
    }
}


pub fn load_rom(path: &str) -> Box<CartridgeAccess> {
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

    let cart = Cartridge::new(PathBuf::from(path), rom, ram_size);

    match cart_type {
        0 => Box::new(CartridgeNoMBC::new(cart)),
        1|2|3 => Box::new(CartridgeMBC1::new(cart)),
        0x13 => Box::new(CartridgeMBC3::new(cart)),
        0x19|0x1b => Box::new(CartridgeMBC5::new(cart)),
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
