
pub struct MMU {
    values: [u8; 65536]
}

impl MMU {
    pub fn new() -> MMU { MMU{ values: [0; 65536] } }

    pub fn read_byte(&self, addr: u16) -> u8 {
        self.values[addr as usize]
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        (self.read_byte(addr) as u16) + ((self.read_byte(addr + 1) as u16) << 8)
    }

    pub fn write_byte(&mut self, addr: u16, byte: u8) -> () {
        self.values[addr as usize] = byte;
    }

    pub fn write_word(&mut self, addr: u16, word: u16) -> () {

    }
}
