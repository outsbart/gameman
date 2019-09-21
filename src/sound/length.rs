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
    timer: u16, // decreases every tick and returns true when it reaches 0
    enable: bool, // if false, the timer isn't clocking

    // keep track if we are in the first half of the length clock period or not
    half_period_passed: bool,
}

impl Length {
    pub fn new(max_length: MaxLength) -> Self {
        Length {
            max_length,
            timer: 0,
            enable: false,
            half_period_passed: false,
        }
    }

    // if true is returned, channel must be disabled
    pub fn tick(&mut self) -> bool {
        self.half_period_passed = false;

        if !self.enabled() {
            return false;
        }

        self.decrease_timer()
    }

    // if true is returned, channel must be disabled
    fn decrease_timer(&mut self) -> bool {
        if self.frozen() {
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

    // informs the length counter that half of the period has reached
    pub fn half_tick(&mut self) {
        self.half_period_passed = true;
    }

    pub fn get_value(&self) -> u16 {
        self.timer
    }

    fn frozen(&self) -> bool {
        self.get_value() == 0
    }

    // returns true if an "unfreeze" was performed
    fn trigger(&mut self) -> bool {
        if self.frozen() {
            self.timer = self.max_length as u16;
            return true;
        }
        false
    }

    // returns true if channel should be disabled
    pub fn set_enable(&mut self, byte: bool, trigger: bool) -> bool {
        let was_disabled = !self.enable;
        let was_frozen = trigger && self.trigger();

        self.enable = byte;

        if !self.enabled() || self.half_period_passed || self.frozen() {
            return false;
        }

        if was_disabled || was_frozen {
            return self.decrease_timer();
        }

        false
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
