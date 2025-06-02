use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};
use std::io;

/// HuC3 (cart type 0xFE) — MBC1-style ROM/RAM banking with RTC and IR.
/// The 0x0000-0x1FFF register selects an operating mode:
///   0x0A = RAM access (normal read/write)
///   0x0B = RTC command interface  → stub: reads return 0x01 (ready), writes ignored
///   0x0C = RTC read               → stub: reads return 0x01, writes ignored
///   0x0D = IR mode                → stub: reads return 0x01, writes ignored
///   other = disabled (0xFF on read)
#[derive(Serialize, Deserialize)]
pub struct CartridgeHuC3 {
    pub(super) cart: Cartridge,
    mode: u8,
}

impl CartridgeHuC3 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart, mode: 0 }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                self.mode = byte & 0x0F;
                if let Err(e) = self.cart.update_ram_enabled(self.mode == 0x0A) {
                    println!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                self.cart.rom_bank = ((byte & 0x7F) as u16).max(1);
            }
            0x4000 | 0x5000 => {
                self.cart.ram_bank = byte & 0x0F;
            }
            0x6000 | 0x7000 => {}
            _ => panic!("Unhandled HuC3 ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        match self.mode {
            0x0A => self.cart.read_ram(addr),
            0x0B | 0x0C => 0x01, // RTC ready/ack stub
            _ => 0xFF,
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.mode == 0x0A {
            self.cart.write_ram(addr, byte);
        }
        // modes 0x0B-0x0D: RTC/IR command, stub = ignore
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}
