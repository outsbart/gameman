use crate::cartridge::{Cartridge, ROM_BANK_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC1Multicart {
    pub(super) cart: Cartridge,
}

impl CartridgeMBC1Multicart {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let cartridge = &self.cart;
        let num_banks = cartridge.rom.len() / ROM_BANK_SIZE;

        // rom_bank uses same format as standard MBC1: bits[6:5]=secondary, bits[4:0]=primary (0→1 applied).
        // Multicart maps secondary×16 (not ×32), and uses only the lower 4 bits of the primary.
        let slot = ((cartridge.rom_bank & 0x60) >> 1) as usize; // secondary × 16
        let primary4 = (cartridge.rom_bank & 0x0F) as usize; // lower 4 bits of primary

        let bank = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => {
                if cartridge.mode == 1 {
                    slot & (num_banks - 1)
                } else {
                    0
                }
            }
            0x4000 | 0x5000 | 0x6000 | 0x7000 => (slot | primary4) & (num_banks - 1),
            _ => panic!("Unhandled ROM MBC1 multicart read at addr {:x}", addr),
        };

        cartridge.rom[bank * ROM_BANK_SIZE + (addr & 0x3FFF) as usize]
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        // Writes are identical to standard MBC1; only the read mapping differs.
        let cartridge = &mut self.cart;
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                if let Err(e) = cartridge.update_ram_enabled(byte & 0x0F == 0x0A) {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                let mut val: u8 = byte & 0x1F;
                if val == 0 {
                    val = 1;
                }
                cartridge.rom_bank = (cartridge.rom_bank & 0x60) + u16::from(val);
            }
            0x4000 | 0x5000 => {
                cartridge.rom_bank = (cartridge.rom_bank & 0x1F) | ((u16::from(byte) & 3) << 5);
                if cartridge.mode == 1 {
                    cartridge.ram_bank = byte & 3;
                }
            }
            0x6000 | 0x7000 => {
                cartridge.mode = byte & 1;
            }
            _ => panic!("Unhandled ROM write at addr 0x{:x}", addr),
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        self.cart.read_ram(addr)
    }
    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        self.cart.write_ram(addr, byte)
    }
}

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC1 {
    pub(super) cart: Cartridge,
}

impl CartridgeMBC1 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        let cartridge = &self.cart;
        let num_banks = cartridge.rom.len() / ROM_BANK_SIZE;

        let bank = match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => {
                if cartridge.mode == 1 {
                    (cartridge.rom_bank & 0x60) as usize & (num_banks - 1)
                } else {
                    0
                }
            }
            0x4000 | 0x5000 | 0x6000 | 0x7000 => cartridge.rom_bank as usize & (num_banks - 1),
            _ => panic!("Unhandled ROM MBC1 read at addr {:x}", addr),
        };

        cartridge.rom[bank * ROM_BANK_SIZE + (addr & 0x3FFF) as usize]
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = &mut self.cart;

        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram
                if let Err(e) = cartridge.update_ram_enabled(byte & 0x0F == 0x0A) {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x2000 | 0x3000 => {
                // change rom bank
                let mut val: u8 = byte & 0x1F;
                if val == 0 {
                    val = 1
                };

                cartridge.rom_bank = (cartridge.rom_bank & 0x60) + u16::from(val);
            }
            0x4000 | 0x5000 => {
                // secondary register: always update rom_bank bits 5-6; in mode 1 also sets ram_bank
                cartridge.rom_bank = (cartridge.rom_bank & 0x1F) | ((u16::from(byte) & 3) << 5);
                if cartridge.mode == 1 {
                    cartridge.ram_bank = byte & 3;
                }
            }
            0x6000 | 0x7000 => {
                cartridge.mode = byte & 1;
            }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        let cart = &self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return 0xFF;
        }
        cart.ram[cart.ram_offset() + addr as usize]
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        let offset = self.cart.ram_offset();
        let cart = &mut self.cart;
        if cart.ram.is_empty() || !cart.ram_enabled {
            return;
        }
        cart.ram[offset + addr as usize] = byte;
        cart.ram_dirty = true;
    }
}
