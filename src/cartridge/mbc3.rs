use crate::cartridge::Cartridge;
use std::io;

pub struct CartridgeMBC3 {
    cart: Cartridge,
    ram_and_timer_enabled: bool,
}

impl CartridgeMBC3 {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            ram_and_timer_enabled: false,
        }
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.cart.read_rom(addr)
    }

    pub fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram and timer
                let was_enabled = self.ram_and_timer_enabled;
                self.ram_and_timer_enabled = byte == 0x0A;
                if was_enabled && !self.ram_and_timer_enabled {
                    if let Err(e) = self.cart.save() {
                        println!("Error saving: {}", e);
                    }
                }
            }
            0x2000 | 0x3000 => {
                // change rom bank
                self.cart.rom_bank = if byte == 0 { 1 } else { byte.into() };
            }
            0x4000 | 0x5000 => {
                // change ram bank or make rtc register readable
                match byte {
                    0x0..=0x3 => {
                        self.cart.mode = 0;
                        self.cart.ram_bank = byte & 3;
                    }
                    0x8..=0xC => self.cart.mode = 1,
                    _ => {}
                }
            }
            0x6000 | 0x7000 => {
                println!("RTC write attempt ignored!")
            }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = &self.cart;

        if cartridge.mode == 1 {
            // return the rtc register value
            println!("attempt to access rtc register");
            return 0x0;
        }
        if cartridge.ram.is_empty() || !self.ram_and_timer_enabled {
            0xFF
        } else {
            let offset = cartridge.ram_bank as usize * crate::cartridge::RAM_BANK_SIZE;
            cartridge.ram[offset + addr as usize]
        }
    }

    pub fn write_ram(&mut self, addr: u16, byte: u8) {
        let ram_and_timer_enabled = self.ram_and_timer_enabled;
        let cartridge = &mut self.cart;

        if cartridge.mode == 1 {
            // write to the rtc register
            println!("attempt to write rtc register");
        }
        if cartridge.ram.is_empty() || !ram_and_timer_enabled {
            return;
        }
        let offset = cartridge.ram_bank as usize * crate::cartridge::RAM_BANK_SIZE;
        cartridge.ram[offset + addr as usize] = byte;
        cartridge.ram_dirty = true;
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.cart.save()
    }
}
