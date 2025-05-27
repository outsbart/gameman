pub mod mbc1;
pub mod mbc2;
pub mod mbc3;
pub mod mbc5;
pub mod nombc;

use crate::cartridge::mbc1::{CartridgeMBC1, CartridgeMBC1Multicart};
use crate::cartridge::mbc2::CartridgeMBC2;
use crate::cartridge::mbc3::CartridgeMBC3;
use crate::cartridge::mbc5::CartridgeMBC5;
use crate::cartridge::nombc::CartridgeNoMBC;

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub const ROM_BANK_SIZE: usize = 0x4000;
pub const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Serialize, Deserialize)]
pub struct Cartridge {
    #[serde(skip)]
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    ram_size: usize,
    ram_enabled: bool,
    ram_dirty: bool,
    rom_bank: u16,
    ram_bank: u8,
    mode: u8,

    path: PathBuf,
    #[serde(skip)]
    save_file: Option<File>,
}

impl Cartridge {
    pub fn new(path: PathBuf, rom: Vec<u8>, ram_size: usize) -> Self {
        let mut cart = Self {
            rom,
            ram: Vec::new(),
            ram_size,
            ram_enabled: false,
            ram_dirty: false,
            rom_bank: 1,
            ram_bank: 0,
            mode: 0,
            path,
            save_file: None,
        };

        if ram_size > 0 {
            match cart.try_load_save_file() {
                Ok(file) => cart.save_file = Some(file),
                Err(e) => {
                    println!("Unable to load/create save file: {}", e)
                }
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
            self.ram = vec![0; self.ram_size];
        } else if file_size != expected_file_size {
            panic!("Save file has unexpected size");
        } else {
            println!("Loading save file");
            file.read_to_end(&mut self.ram)?;
        };

        Ok(file)
    }

    pub fn update_ram_enabled(&mut self, enabled: bool) -> io::Result<()> {
        let was_enabled = self.ram_enabled;
        self.ram_enabled = enabled;
        if was_enabled && !self.ram_enabled {
            self.save()?;
        }
        Ok(())
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn save(&mut self) -> io::Result<()> {
        if self.ram_dirty
            && let Some(file) = self.save_file.as_mut()
        {
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&self.ram)?;
            self.ram_dirty = false;
        }
        Ok(())
    }

    pub fn restore(&mut self) -> io::Result<()> {
        self.rom = std::fs::read(&self.path)?;
        if self.ram_size > 0 {
            if let Ok(file) = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(self.save_file_path())
            {
                self.save_file = Some(file);
            }
        }
        Ok(())
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let abs_addr = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => addr as usize,
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                let num_banks = self.rom.len() / ROM_BANK_SIZE;
                (self.rom_bank as usize & (num_banks - 1)) * ROM_BANK_SIZE
                    + (addr & 0x3FFF) as usize
            }
            _ => panic!("Unhandled ROM MBC read at addr {:x}", addr),
        };
        if abs_addr < self.rom.len() {
            self.rom[abs_addr]
        } else {
            0
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        let offset = self.ram_bank as usize * RAM_BANK_SIZE;
        if self.ram.is_empty() || !self.ram_enabled {
            0xFF
        } else {
            self.ram[offset + addr as usize]
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        let offset = self.ram_bank as usize * RAM_BANK_SIZE;
        if self.ram.is_empty() || !self.ram_enabled {
            return;
        }
        self.ram[offset + addr as usize] = byte;
        self.ram_dirty = true;
    }
}

macro_rules! dispatch_cartridge {
    ($self:expr, $method:ident ( $($arg:expr),* )) => {
        match $self {
            CartridgeKind::NoMBC(c) => c.$method($($arg),*),
            CartridgeKind::MBC1(c) => c.$method($($arg),*),
            CartridgeKind::MBC1Multicart(c) => c.$method($($arg),*),
            CartridgeKind::MBC2(c) => c.$method($($arg),*),
            CartridgeKind::MBC3(c) => c.$method($($arg),*),
            CartridgeKind::MBC5(c) => c.$method($($arg),*),
        }
    };
}

#[derive(Serialize, Deserialize)]
pub enum CartridgeKind {
    NoMBC(CartridgeNoMBC),
    MBC1(CartridgeMBC1),
    MBC1Multicart(CartridgeMBC1Multicart),
    MBC2(CartridgeMBC2),
    MBC3(CartridgeMBC3),
    MBC5(CartridgeMBC5),
}

impl CartridgeKind {
    pub fn read_rom(&self, addr: u16) -> u8 {
        dispatch_cartridge!(self, read_rom(addr))
    }
    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        dispatch_cartridge!(self, write_rom(addr, byte))
    }
    pub fn read_ram(&self, addr: u16) -> u8 {
        dispatch_cartridge!(self, read_ram(addr))
    }
    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        dispatch_cartridge!(self, write_ram(addr, byte))
    }
    pub fn save(&mut self) -> io::Result<()> {
        dispatch_cartridge!(self, save())
    }

    pub fn inner_cart(&self) -> &Cartridge {
        match self {
            CartridgeKind::NoMBC(c) => &c.cart,
            CartridgeKind::MBC1(c) => &c.cart,
            CartridgeKind::MBC1Multicart(c) => &c.cart,
            CartridgeKind::MBC2(c) => &c.cart,
            CartridgeKind::MBC3(c) => &c.cart,
            CartridgeKind::MBC5(c) => &c.cart,
        }
    }

    pub fn inner_cart_mut(&mut self) -> &mut Cartridge {
        match self {
            CartridgeKind::NoMBC(c) => &mut c.cart,
            CartridgeKind::MBC1(c) => &mut c.cart,
            CartridgeKind::MBC1Multicart(c) => &mut c.cart,
            CartridgeKind::MBC2(c) => &mut c.cart,
            CartridgeKind::MBC3(c) => &mut c.cart,
            CartridgeKind::MBC5(c) => &mut c.cart,
        }
    }

    pub fn restore(&mut self) -> io::Result<()> {
        self.inner_cart_mut().restore()
    }

    pub fn rtc_base_secs_mut(&mut self) -> Option<&mut u64> {
        match self {
            CartridgeKind::MBC3(c) => c.rtc.as_mut().map(|r| &mut r.base_unix_secs),
            _ => None,
        }
    }
}

fn is_mbc1_multicart(rom: &[u8]) -> bool {
    // Heuristic: 64-bank (1 MB) MBC1 ROM where every bank carries a Nintendo logo.
    // Real multicarts embed the logo in each slot header so the boot ROM passes;
    // standard single-game ROMs only have the logo in bank 0.
    if rom.len() / ROM_BANK_SIZE != 64 {
        return false;
    }
    let logo_prefix: [u8; 4] = [0xCE, 0xED, 0x66, 0x66];
    (0..64).all(|bank| {
        let off = bank * ROM_BANK_SIZE + 0x104;
        rom[off..off + 4] == logo_prefix
    })
}

pub fn load_rom(path: &str) -> CartridgeKind {
    let mut rom: Vec<u8> = Vec::new();

    match File::open(path) {
        Ok(mut file) => {
            match file.read_to_end(&mut rom) {
                Ok(_) => {}
                Err(_) => panic!("couldnt read the rom into the buffer!"),
            };
        }
        Err(_) => panic!("couldnt open the rom file"),
    }

    let cart_type = rom[0x147] as usize;

    let ram_size = if cart_type == 0x05 || cart_type == 0x06 {
        // MBC2 has 512 × 4-bit internal RAM; header always reports 0 external RAM
        512
    } else {
        (match rom[0x149] {
            0x00 => 0,
            0x01 => 2,
            0x02 => 8,
            0x03 => 32,
            0x04 => 128,
            0x05 => 64,
            _ => panic!("Unrecognized cartridge ram size"),
        }) * 1024
    };

    println!("rom size = 0x{:x}", rom.len());
    println!("rom type = 0x{:x}", cart_type);
    println!("ram size = 0x{:x}", ram_size);

    let multicart = (1..=3).contains(&cart_type) && is_mbc1_multicart(&rom);
    let cart = Cartridge::new(PathBuf::from(path), rom, ram_size);

    match cart_type {
        0 => CartridgeKind::NoMBC(CartridgeNoMBC::new(cart)),
        1..=3 => {
            if multicart {
                CartridgeKind::MBC1Multicart(CartridgeMBC1Multicart::new(cart))
            } else {
                CartridgeKind::MBC1(CartridgeMBC1::new(cart))
            }
        }
        0x05 | 0x06 => CartridgeKind::MBC2(CartridgeMBC2::new(cart)),
        0x0F..=0x13 => {
            let has_rtc = matches!(cart_type, 0x0F | 0x10);
            CartridgeKind::MBC3(CartridgeMBC3::new(cart, has_rtc))
        }
        0x19..=0x1E => CartridgeKind::MBC5(CartridgeMBC5::new(cart)),
        _ => panic!("Cartridge type {:x} not implemented", cart_type),
    }
}
