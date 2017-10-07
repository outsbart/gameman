pub struct MMU {      // TODO: fix the sized when tests are implemented
    still_bios: bool, bios: [u8; 65536],

    rom: [u8; 65536], wram: [u8; 65536],
    eram: [u8; 65536], zram: [u8; 65536],
}

impl MMU {
    pub fn new() -> MMU {
        MMU {
            still_bios: true, bios: [0; 65536],

            rom: [0; 65536], wram: [0; 65536],
            eram: [0; 65536], zram: [0; 65536]
        }
    }
}


pub trait Memory {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);

    fn read_word(&mut self, addr: u16) -> u16 {
        (self.read_byte(addr) as u16) | ((self.read_byte(addr + 1) as u16) << 8)
    }

    fn write_word(&mut self, addr: u16, word: u16) -> () {
        self.write_byte(addr, (word & 0x00FF) as u8);
        self.write_byte(addr + 1, ((word & 0xFF00) >> 8) as u8);
    }
}


impl Memory for MMU {
    fn read_byte(&mut self, addr: u16) -> u8 {

        // TODO: once everything works and is tested, refactor using actual ranges
        match addr & 0xF000 {
            // BIOS
            0x0000 => {
                if self.still_bios {
                    if addr < 0x0100 {
                        return self.bios[addr as usize]
                    }
                    else if addr == 0x0100 {
                        self.still_bios = false;    // TODO: check if it actually works
                    }
                }
                return self.rom[addr as usize]
            }

            0x1000...0x3000 => { return self.rom[addr as usize] }             // ROM 0
            0x4000...0x7000 => { return self.rom[addr as usize] }             // ROM 1
            // VRAM
            0x8000 | 0x9000 => { return self.rom[(addr &0x1FFF) as usize] }     // TODO: change when GPU impl
            0xA000 | 0xB000 => { return self.eram[(addr &0x1FFF) as usize] }    // External RAM
            0xC000...0xE000 => { return self.wram[(addr &0x1FFF) as usize] }    // Working RAM

            // Working RAM shadow, GPU OAM, I/O, Zero-page RAM
            0xF000 => {
                match addr & 0x0F00 {
                    0x0000...0x0D00 => { return self.wram[(addr & 0x1FFF) as usize] } // Working RAM echo

                    // GPU OAM
                    0x0E00 => {
                        if addr < 0xFEA0 { return self.rom[(addr & 0x00FF) as usize] }    // TODO: change when GPU implemented
                        else { return 0 }
                    }

                    // Zero page
                    0x0F00 => {
                        if addr >= 0xFF80 { return self.zram[(addr & 0x007F) as usize] }
                        else { return 0 }  // TODO: change when IO implemented
                    }

                    _ => {
                        panic!("Unhandled memory access")
                    }
                }
            }

            _ => {
                panic!("Unhandled memory access")
            }

        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        self.rom[addr as usize] = byte;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian() {
        let mut mmu = MMU::new();

        mmu.write_word(0x1000, 0x1FF);
        assert_eq!(0x1FF, mmu.read_word(0x1000))
    }

    #[test]
    fn read_and_write_byte() {
        let mut mmu = MMU::new();

        mmu.write_byte(0x1000, 0x1);
        assert_eq!(0x1, mmu.read_byte(0x1000))
    }

    /// after instruction 0x0100 is reached,
    /// for addresses < 0x0100, rom should be accessed instead of bios
    #[test]
    fn bios_gets_replaced_by_rom() {
        let mut mmu = MMU::new();

        mmu.rom[0x00FF as usize] = 5;
        mmu.rom[0x0100 as usize] = 6;
        mmu.bios[0x00FF as usize] = 3;
        mmu.bios[0x0100 as usize] = 4;

        assert_eq!(mmu.read_byte(0x00FF), 3);
        assert_eq!(mmu.read_byte(0x0100), 6);
        assert_eq!(mmu.read_byte(0x00FF), 5);
    }

    /// test succesful mapping for eram access
    /// from 0xA000 to 0xBFFF should access eram
    #[test]
    fn eram_access() {
        let mut mmu = MMU::new();

        mmu.eram = [1; 65536];

        assert_eq!(mmu.read_byte(0x0FFF), 0);
        assert_eq!(mmu.read_byte(0xA000), 1);
        assert_eq!(mmu.read_byte(0xB000), 1);
        assert_eq!(mmu.read_byte(0xBFFF), 1);
        assert_eq!(mmu.read_byte(0xC000), 0);
    }

    /// test succesful mapping for wram access
    /// from 0xC000 to 0xFDFF should access wram
    #[test]
    fn wram_access() {
        let mut mmu = MMU::new();

        mmu.wram = [1; 65536];

        assert_eq!(mmu.read_byte(0xBFFF), 0);
        assert_eq!(mmu.read_byte(0xC000), 1);
        assert_eq!(mmu.read_byte(0xD000), 1);
        assert_eq!(mmu.read_byte(0xE000), 1);
        assert_eq!(mmu.read_byte(0xFDFF), 1);
        assert_eq!(mmu.read_byte(0xFE00), 0);
    }

    /// test succesful mapping for zero ram access
    /// from 0xFF80 to 0xFFFF should access zero ram
    #[test]
    fn zram_access() {
        let mut mmu = MMU::new();

        mmu.zram = [1; 65536];

        assert_eq!(mmu.read_byte(0xFF7F), 0);
        assert_eq!(mmu.read_byte(0xFF80), 1);
        assert_eq!(mmu.read_byte(0xFFFF), 1);
    }
}