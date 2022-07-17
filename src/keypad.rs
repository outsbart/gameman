pub struct Key {
    rows: [u8; 2],
    column: u8,
}

pub enum Button {
    DOWN,
    UP,
    LEFT,
    RIGHT,
    START,
    SELECT,
    B,
    A,
}

impl Key {
    pub fn new() -> Key {
        Key {
            rows: [0xCF, 0xCF],
            column: 0,
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        (match self.column {
            0x10 => self.rows[0],
            0x20 => self.rows[1],
            _ => 0xCF,
        } | self.column)
    }

    pub fn write_byte(&mut self, value: u8) {
        self.column = value & 0b110000;
    }

    pub fn press(&mut self, button: Button) {
        match button {
            Button::DOWN => self.rows[1] &= 0xC7,
            Button::UP => self.rows[1] &= 0xCB,
            Button::LEFT => self.rows[1] &= 0xCD,
            Button::RIGHT => self.rows[1] &= 0xCE,
            Button::START => self.rows[0] &= 0xC7,
            Button::SELECT => self.rows[0] &= 0xCB,
            Button::B => self.rows[0] &= 0xCD,
            Button::A => self.rows[0] &= 0xCE,
        }
    }

    pub fn release(&mut self, button: Button) {
        match button {
            Button::DOWN => self.rows[1] |= 0x8,
            Button::UP => self.rows[1] |= 0x4,
            Button::LEFT => self.rows[1] |= 0x2,
            Button::RIGHT => self.rows[1] |= 0x1,
            Button::START => self.rows[0] |= 0x8,
            Button::SELECT => self.rows[0] |= 0x4,
            Button::B => self.rows[0] |= 0x2,
            Button::A => self.rows[0] |= 0x1,
        }
    }
}

impl Default for Key {
    fn default() -> Self {
        Key::new()
    }
}
