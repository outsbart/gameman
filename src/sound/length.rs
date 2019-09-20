// used to shut off a channel after a period of time
pub struct Length {
    timer: u8,  // it's not a Timer type because it doesn't automatically reset when it reaches 0
    enable: bool,
}

impl Length {
    pub fn new() -> Self {
        Length {
            timer: 0,
            enable: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.timer = self.timer.wrapping_sub(1);

        // if timer has run out
        if self.timer == 0 {
            return true;
        }

        false
    }

    pub fn set_value(&mut self, value: u8) {
        self.timer = value;
    }

    pub fn get_value(&self) -> u8 {
        self.timer
    }

    pub fn set_enable(&mut self, byte: bool) {
        self.enable = byte;
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
