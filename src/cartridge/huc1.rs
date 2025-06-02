use crate::cartridge::{Cartridge, RAM_BANK_SIZE, ROM_BANK_SIZE};
use serde::{Deserialize, Serialize};
use std::io;

/// HuC1 (cart type 0xFF) — MBC1-compatible banking with an IR LED/receiver.
/// The only behavioral difference from MBC1 is the 0x0000-0x1FFF register:
/// writing 0x0A enables normal RAM access; anything else enables IR mode,
/// which we stub (reads return 0xFF, writes are ignored).
#[derive(Serialize, Deserialize)]
pub struct CartridgeHuC1 {
    pub(super) cart: Cartridge,
    ir_mode: bool,
}

impl CartridgeHuC1 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ir_mode: true,
        }
    }

    fn ram_offset(&self) -> usize {
        let cart = &self.cart;
        if cart.mode == 0 || cart.ram.is_empty() {
            return 0;
        }
        let num_ram_banks = cart.ram.len() / RAM_BANK_SIZE;
        (cart.ram_bank as usize & (num_ram_banks - 1)) * RAM_BANK_SIZE
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let cart = &self.cart;
        let num_banks = cart.rom.len() / ROM_BANK_SIZE;
        let bank = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => {
                if cart.mode == 1 {
                    (cart.rom_bank & 0x60) as usize & (num_banks - 1)
                } else {
                    0
                }
            }
            0x4000 | 0x5000 | 0x6000 | 0x7000 => cart.rom_bank as usize & (num_banks - 1),
            _ => panic!("Unhandled HuC1 ROM read at addr {:x}", addr),
        };
        cart.rom[bank * ROM_BANK_SIZE + (addr & 0x3FFF) as usize]
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                let ram_enable = byte == 0x0A;
                self.ir_mode = !ram_enable;
                if let Err(e) = self.cart.update_ram_enabled(ram_enable) {
                    println!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                let val = (byte & 0x1F).max(1);
                self.cart.rom_bank = (self.cart.rom_bank & 0x60) | val as u16;
            }
            0x4000 | 0x5000 => {
                self.cart.rom_bank = (self.cart.rom_bank & 0x1F) | ((byte as u16 & 0x03) << 5);
                if self.cart.mode == 1 {
                    self.cart.ram_bank = byte & 0x03;
                }
            }
            0x6000 | 0x7000 => {
                self.cart.mode = byte & 0x01;
            }
            _ => panic!("Unhandled HuC1 ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if self.ir_mode {
            return 0xFF;
        }
        let cart = &self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return 0xFF;
        }
        cart.ram[self.ram_offset() + addr as usize]
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.ir_mode {
            return;
        }
        let offset = self.ram_offset();
        let cart = &mut self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return;
        }
        cart.ram[offset + addr as usize] = byte;
        cart.ram_dirty = true;
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}
