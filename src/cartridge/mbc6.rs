use crate::cartridge::{Cartridge, ROM_BANK_SIZE};
use serde::{Deserialize, Serialize};

// MBC6 (cart type 0x20) — Net de Get: Mini-Game @ 100.
//
// Unusually, *both* 16 KB ROM windows are independently bankable.
// External RAM is 8 KB split into two 4 KB halves, each with its own bank select.
// The cartridge also contains flash memory (for downloading mini-games) but that
// is not emulated here; flash writes are silently ignored.
//
// Register map (writes to ROM area):
//   0x0000–0x0FFF  RAM enable        (0x0A = enable)
//   0x1000–0x17FF  SRAM bank A       (bit 0)
//   0x1800–0x1FFF  SRAM bank B       (bit 0)
//   0x2000–0x27FF  ROM bank A bits 0–6
//   0x2800–0x2FFF  ROM bank A bit 7
//   0x3000–0x37FF  ROM bank B bits 0–6
//   0x3800–0x3FFF  ROM bank B bit 7
//   0x4000–0x7FFF  Flash mode        (ignored)

const SRAM_HALF: usize = 0x1000; // 4 KB per RAM half

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC6 {
    pub(super) cart: Cartridge,
    rom_bank_a: u8, // bank mapped to 0x0000–0x3FFF
    rom_bank_b: u8, // bank mapped to 0x4000–0x7FFF
    sram_bank_a: u8,
    sram_bank_b: u8,
}

impl CartridgeMBC6 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            rom_bank_a: 0,
            rom_bank_b: 1,
            sram_bank_a: 0,
            sram_bank_b: 1,
        }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        // Both halves use their own independently-selected bank.
        let (bank, offset) = if addr < 0x4000 {
            (self.rom_bank_a as usize, addr as usize)
        } else {
            (self.rom_bank_b as usize, (addr - 0x4000) as usize)
        };
        let num_banks = self.cart.rom.len() / ROM_BANK_SIZE;
        let abs = (bank & (num_banks - 1)) * ROM_BANK_SIZE + offset;
        if abs < self.cart.rom.len() {
            self.cart.rom[abs]
        } else {
            0
        }
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF800 {
            0x0000 => {
                if let Err(e) = self.cart.update_ram_enabled(byte == 0x0A) {
                    log::warn!("Error saving: {}", e);
                }
            }
            0x1000 => self.sram_bank_a = byte & 0x01,
            0x1800 => self.sram_bank_b = byte & 0x01,
            0x2000 => {
                // low 7 bits of ROM bank A
                self.rom_bank_a = (self.rom_bank_a & 0x80) | (byte & 0x7F);
            }
            0x2800 => {
                // bit 7 of ROM bank A
                self.rom_bank_a = (self.rom_bank_a & 0x7F) | ((byte & 0x01) << 7);
            }
            0x3000 => {
                // low 7 bits of ROM bank B
                self.rom_bank_b = (self.rom_bank_b & 0x80) | (byte & 0x7F);
            }
            0x3800 => {
                // bit 7 of ROM bank B
                self.rom_bank_b = (self.rom_bank_b & 0x7F) | ((byte & 0x01) << 7);
            }
            0x4000..=0x7800 => {} // flash enable/mode — not implemented
            _ => {}
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        if self.cart.ram.is_empty() || !self.cart.ram_enabled {
            return 0xFF;
        }
        // 0x0000–0x0FFF → SRAM bank A (4 KB halves)
        // 0x1000–0x1FFF → SRAM bank B
        let (bank, offset) = if addr < 0x1000 {
            (self.sram_bank_a as usize, addr as usize)
        } else {
            (self.sram_bank_b as usize, (addr - 0x1000) as usize)
        };
        let abs = bank * SRAM_HALF + offset;
        if abs < self.cart.ram.len() {
            self.cart.ram[abs]
        } else {
            0xFF
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.cart.ram.is_empty() || !self.cart.ram_enabled {
            return;
        }
        let (bank, offset) = if addr < 0x1000 {
            (self.sram_bank_a as usize, addr as usize)
        } else {
            (self.sram_bank_b as usize, (addr - 0x1000) as usize)
        };
        let abs = bank * SRAM_HALF + offset;
        if abs < self.cart.ram.len() {
            self.cart.ram[abs] = byte;
            self.cart.ram_dirty = true;
        }
    }
}
