use sound::Timer;

// used to shut off a channel after a period of time
pub struct Length {
    timer: Timer,
    value: u8,
    enable: bool,
}

impl Length {
    pub fn new() -> Self {
        Length {
            timer: Timer::new(64),
            value: 0,
            enable: false,
        }
    }

    pub fn tick(&mut self) {
        if !self.enable {
            return;
        }

        // if timer has run out
        if self.timer.tick() {
            self.enable = false;
        }
    }

    pub fn set_value(&mut self, value: u8) {
        self.value = value;
    }

    pub fn get_value(&self) -> u8 {
        self.value
    }

    pub fn set_enable(&mut self, value: bool) {
        self.enable = value;
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
