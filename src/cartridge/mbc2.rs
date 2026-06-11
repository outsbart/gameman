use crate::cartridge::Cartridge;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CartridgeMBC2 {
    pub(super) cart: Cartridge,
}

impl CartridgeMBC2 {
    pub fn new(cart: Cartridge) -> Self {
        Self { cart }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        if addr > 0x3FFF {
            return;
        }

        let cartridge = &mut self.cart;

        if addr & 0x0100 == 0 {
            if let Err(e) = cartridge.update_ram_enabled(byte & 0x0F == 0x0A) {
                log::warn!("Error saving: {}", e);
            }
        } else {
            let mut bank = byte & 0x0F;
            if bank == 0 {
                bank = 1;
            }
            cartridge.rom_bank = bank as u16;
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = &self.cart;

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return 0xFF;
        }

        cartridge.ram[(addr & 0x01FF) as usize] | 0xF0
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        let cartridge = &mut self.cart;

        if cartridge.ram.is_empty() || !cartridge.ram_enabled {
            return;
        }

        cartridge.ram[(addr & 0x01FF) as usize] = byte & 0x0F;
        cartridge.ram_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::{Cartridge, ROM_BANK_SIZE};
    use std::path::PathBuf;

    fn make_mbc2(num_rom_banks: usize) -> CartridgeMBC2 {
        let mut rom = vec![0u8; num_rom_banks * ROM_BANK_SIZE];
        for b in 0..num_rom_banks {
            rom[b * ROM_BANK_SIZE + 0x100] = b as u8;
        }
        let cart = Cartridge::new(PathBuf::from("test.gb"), rom, 0);
        CartridgeMBC2::new(cart)
    }

    #[test]
    fn bit8_zero_enables_ram() {
        let mut c = make_mbc2(2);
        c.cart.ram = vec![0u8; 512];
        c.cart.ram[0] = 0x03;

        // addr bit-8 = 1 → ROM bank select, not RAM enable
        c.write_rom(0x0100, 0x0A);
        assert_eq!(c.read_ram(0), 0xFF); // RAM still disabled

        // addr bit-8 = 0 → RAM enable
        c.write_rom(0x0000, 0x0A);
        assert_eq!(c.read_ram(0), 0x03 | 0xF0); // 0xF3
    }

    #[test]
    fn bit8_one_selects_rom_bank() {
        let mut c = make_mbc2(4);

        // addr bit-8 = 1 → ROM bank select
        c.write_rom(0x0100, 0x03);
        assert_eq!(c.read_rom(0x4100), 3);

        // addr bit-8 = 0 → RAM enable/disable, bank unchanged
        c.write_rom(0x0000, 0x00);
        assert_eq!(c.read_rom(0x4100), 3); // bank still 3
    }

    #[test]
    fn ram_upper_nibble_is_forced_high() {
        let mut c = make_mbc2(2);
        c.cart.ram = vec![0u8; 512];
        c.write_rom(0x0000, 0x0A); // enable RAM
        c.write_ram(0, 0x55); // stored as 0x05 (& 0x0F)
        assert_eq!(c.read_ram(0), 0x05 | 0xF0); // 0xF5
        c.write_ram(1, 0x0F);
        assert_eq!(c.read_ram(1), 0xFF); // 0x0F | 0xF0
    }

    #[test]
    fn ram_address_wraps_at_512_bytes() {
        let mut c = make_mbc2(2);
        c.cart.ram = vec![0u8; 512];
        c.write_rom(0x0000, 0x0A); // enable RAM
        c.write_ram(0x0000, 0x07);
        // 0x0200 & 0x01FF = 0x0000 → same byte
        assert_eq!(c.read_ram(0x0200), 0x07 | 0xF0);
    }
}
