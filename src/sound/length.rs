// all the channels have a max length value of 64, except for wave
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum MaxLength {
    Wave = 256,
    NotWave = 64,
}

impl Into<u16> for MaxLength {
    fn into(self) -> u16 {
        match self {
            MaxLength::Wave => 256,
            MaxLength::NotWave => 64,
        }
    }
}

// used to shut off a channel after a period of time
pub struct Length {
    max_length: MaxLength, // the max value that the length can have
    timer: u16,  // decreases every tick and returns true when it reaches 0
    enable: bool,
}

impl Length {
    pub fn new(max_length: MaxLength) -> Self {
        Length {
            max_length,
            timer: 0,
            enable: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        if !self.enabled() {
            return false;
        }

        self.timer = self.timer.wrapping_sub(1);

        // if timer has run out
        if self.timer == 0 {
            return true;
        }

        false
    }

    pub fn set_value(&mut self, value: u8) {
        let max_val = self.max_length as u16;

        self.timer = match self.max_length {
            MaxLength::NotWave => {
                max_val - ((value as u16) & (max_val - 1))
            },
            MaxLength::Wave => {
                max_val - (value as u16)
            }
        }
    }

    pub fn get_value(&self) -> u16 {
        self.timer
    }

    pub fn set_to_max(&mut self) {
        self.timer = self.max_length as u16;
    }

    pub fn set_enable(&mut self, byte: bool) {
        self.enable = byte;
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
