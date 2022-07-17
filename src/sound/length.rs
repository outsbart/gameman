// all the channels have a max length value of 64, except for wave
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum MaxLength {
    Wave = 256,
    NotWave = 64,
}

impl From<u16> for MaxLength {
    fn from(val: u16) -> Self {
        match val {
            64 => MaxLength::NotWave,
            256 => MaxLength::Wave,
            _ => panic!("Impossible value"),
        }
    }
}

// used to shut off a channel after a period of time
pub struct Length {
    max_length: MaxLength, // the max value that the length can have
    enable: bool,          // is length enabled? if not, clocking won't affect length
    timer: u16,            // decreases every tick if enable = true

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

    // returns true if length freezes (aka timer becomes zero)
    // and therefore channel must be disabled
    fn decrease_timer(&mut self) -> bool {
        // if already frozen, do nothing
        if self.frozen() {
            return false;
        }

        // clock
        self.timer = self.timer.wrapping_sub(1);

        self.frozen()
    }

    pub fn set_value(&mut self, value: u8) {
        let max_val = self.max_length as u16;

        self.timer = match self.max_length {
            MaxLength::NotWave => max_val - ((value as u16) & (max_val - 1)),
            MaxLength::Wave => max_val - (value as u16),
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

    // returns true if length freezes (aka timer becomes zero)
    // and therefore channel must be disabled
    pub fn set_enable(&mut self, byte: bool, trigger: bool) -> bool {
        let was_disabled = !self.enable;

        self.enable = byte;

        if was_disabled && self.enabled() && !self.half_period_passed {
            self.decrease_timer();
        }

        let was_frozen = trigger && self.trigger();

        if was_frozen && self.enabled() && !self.half_period_passed {
            self.decrease_timer();
        }

        self.frozen()
    }

    pub fn enabled(&self) -> bool {
        self.enable
    }
}
