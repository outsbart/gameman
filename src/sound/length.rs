use sound::Timer;

// used to shut off a channel after a period of time
pub struct Length {
    timer: Timer,
    enable: bool,
}

impl Length {
    pub fn new() -> Self {
        Length {
            timer: Timer::new(64 * 0x4000),
            enable: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        if !self.enable {
            return false;
        }

        // if timer has run out
        if self.timer.tick() {
            return true;
        }

        false
    }

    pub fn set_value(&mut self, value: u8) {
        self.timer.curr = value as usize;
    }

    pub fn get_value(&self) -> u8 {
        self.timer.curr as u8
    }

    pub fn set_enable(&mut self, byte: bool) {
        self.enable = byte;
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
