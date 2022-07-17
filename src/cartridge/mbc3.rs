use cartridge::{Cartridge, CartridgeAccess};

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
}

impl CartridgeAccess for CartridgeMBC3 {
    fn cartridge(&self) -> &Cartridge {
        &self.cart
    }
    fn cartridge_mut(&mut self) -> &mut Cartridge {
        &mut self.cart
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        let cartridge = self.cartridge_mut();

        match addr & 0xF000 {
            0x0000 | 0x1000 => {
                // enable eram and timer
                self.ram_and_timer_enabled = byte == 0x0A;
            }
            0x2000 | 0x3000 => {
                // change rom bank
                cartridge.rom_bank = if byte == 0 { 1 } else { byte.into() };
            }
            0x4000 | 0x5000 => {
                // change ram bank or make rtc register readable
                match byte {
                    0x0..=0x3 => {
                        cartridge.mode = 0;
                        cartridge.ram_bank = byte & 3;
                    }
                    0x8..=0xC => cartridge.mode = 1,
                    _ => {}
                }
            }
            0x6000 | 0x7000 => {
                println!("RTC write attempt ignored!")
            }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr),
        };
    }

    fn read_ram(&self, addr: u16) -> u8 {
        let cartridge = self.cartridge();

        if cartridge.mode == 1 {
            // return the rtc register value
            println!("attempt to access rtc register");
            return 0x0;
        }
        if cartridge.ram.is_empty() || !self.ram_and_timer_enabled {
            return 0xFF;
        } else {
            return cartridge.ram[self.ram_offset() + addr as usize];
        }
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        let ram_and_timer_enabled = self.ram_and_timer_enabled;
        let ram_offset = self.ram_offset();

        let cartridge = self.cartridge_mut();

        if cartridge.mode == 1 {
            // write to the rtc register
            println!("attempt to write rtc register");
        }
        if cartridge.ram.is_empty() || !ram_and_timer_enabled {
            return;
        }
        cartridge.ram[ram_offset + addr as usize] = byte;
    }
}
