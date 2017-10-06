
pub struct MMU {
    values: [u8; 65536]
}

impl MMU {
    pub fn new() -> MMU { MMU{ values: [0; 65536] } }
}


pub trait Memory {
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);

    fn read_word(&self, addr: u16) -> u16 {
        (self.read_byte(addr) as u16) | ((self.read_byte(addr + 1) as u16) << 8)
    }

    fn write_word(&mut self, addr: u16, word: u16) -> () {
        self.write_byte(addr, (word & 0xFF) as u8);
        self.write_byte(addr + 1, ((word & 0xFF00) >> 8) as u8);
    }
}


impl Memory for MMU {
    fn read_byte(&self, addr: u16) -> u8 {
        self.values[addr as usize]
    }

    fn write_byte(&mut self, addr: u16, byte: u8) {
        self.values[addr as usize] = byte;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian() {
        let mut mmu = MMU::new();

        mmu.write_word(0xF0, 0x1FF);
        assert_eq!(0x1FF, mmu.read_word(0xF0))
    }

    #[test]
    fn read_and_write_byte() {
        let mut mmu = MMU::new();

        mmu.write_byte(0xF0, 0x1);
        assert_eq!(0x1, mmu.read_byte(0xF0))
    }
}