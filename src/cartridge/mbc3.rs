use cartridge::Cartridge;

pub struct CartridgeMBC3 {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,

    ram_and_timer_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    rom_offset: usize,
    ram_offset: usize,
    mode: u8,
}

impl CartridgeMBC3 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        CartridgeMBC3 {
            rom, ram: vec![0; ram_size],
            ram_and_timer_enabled: false,
            rom_bank: 1, rom_offset: 0x4000,
            ram_bank: 0, ram_offset: 0,
            mode: 0,
        }
    }
}

impl Cartridge for CartridgeMBC3 {
    fn read_rom(&mut self, addr: u16) -> u8 {
        match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => self.rom[addr as usize],
            0x4000 | 0x5000 | 0x6000 | 0x7000 => {
                self.rom[self.rom_offset + (addr & 0x3FFF) as usize]
            }
            _ => panic!("Unhandled ROM MBC1 read at addr {:x}", addr)
        }
    }

    fn write_rom(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000 | 0x1000 => {  // enable eram and timer
                self.ram_and_timer_enabled = byte == 0x0A;
            }
            0x2000 | 0x3000 => {  // change rom bank
                self.rom_bank = if byte == 0 { 1 } else { byte };
                self.rom_offset = self.rom_bank as usize * 0x4000;
            }
            0x4000 | 0x5000 => {  // change ram bank or make rtc register readable
                match byte {
                    0x0..=0x3 => {
                        self.mode = 0;
                        self.ram_bank = byte & 3;
                        self.ram_offset = self.ram_bank as usize * 0x2000;
                    }
                    0x8..=0xC => {
                        self.mode = 1
                    }
                    _ => {}
                }
            }
            0x6000 | 0x7000 => { println!("RTC write attempt ignored!") }
            _ => panic!("Unhandled rom write at addr 0x{:x}", addr)
        };
    }

    fn read_ram(&mut self, addr: u16) -> u8 {
        if self.mode == 1 {  // return the rtc register value
            println!("attempt to access rtc register");
            return 0x0
        }
        if self.ram.is_empty() || !self.ram_and_timer_enabled {
            return 0xFF
        } else {
            return self.ram[self.ram_offset + addr as usize]
        }
    }

    fn write_ram(&mut self, addr: u16, byte: u8) {
        if self.mode == 1 {  // write to the rtc register
            println!("attempt to write rtc register");
        }
        if self.ram.is_empty() || !self.ram_and_timer_enabled {
            return
        }
        self.ram[self.ram_offset + addr as usize] = byte;
    }
}
