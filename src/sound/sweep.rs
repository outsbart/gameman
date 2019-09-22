use sound::{TimerDefaultPeriod};


pub struct Sweep {
    pub shift: u8,
    negate: bool, // true if the shifted value should be negated during sweep calculation
    pub timer: TimerDefaultPeriod,
    shadow_frequency: u16,
    enabled: bool,
}

impl Sweep {
    pub fn new() -> Self {
        Sweep {
            shift: 0,
            negate: false,
            timer: TimerDefaultPeriod::new(),
            shadow_frequency: 0,
            enabled: false,
        }
    }

    pub fn write(&mut self, value: u8) {
        self.shift = value & 0b0000_0111;
        self.negate = value & 0b1000 != 0;
        self.timer.period = ((value & 0b0111_0000) >> 4) as usize;
        self.timer.restart();
    }

    pub fn read(&self) -> u8 {
        0b1000_0000 |
        ((self.timer.period as u8) << 4) |
        (if self.negate {0b1000} else {0}) |
        self.shift
    }

    // return true if frequency calculations should be performed immediately
    pub fn trigger(&mut self, freq: u16) -> bool {
        // During a trigger event, several things occur:
        // - The internal enabled flag is set if either the sweep period or shift
        //   are non-zero, cleared otherwise.
        // - If the sweep shift is non-zero, frequency calculation and the overflow
        //   check are performed immediately.

        // - Square 1's frequency is copied to the shadow register.
        self.shadow_frequency = freq;

        // - The sweep timer is reloaded.
        self.timer.restart();

        self.enabled = (self.timer.period > 0) || (self.shift > 0);

        self.shift > 0
    }

    // calculates the sweep, returns the new freq value and whether there was an overflow
    pub fn calculate(&mut self) -> (u16, bool) {
        let shifted = self.shadow_frequency >> self.shift as u16;

        let mut result = self.shadow_frequency;

        if !self.negate {
            result += shifted;
        } else {
            result -= shifted;
        }

        (result & 0b111_1111_1111, result > 0x7FF)
    }

    pub fn set_shadow_frequency(&mut self, freq: u16) {
        self.shadow_frequency = freq;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_read_write() {
        let mut sweep: Sweep = Sweep::new();
        assert_eq!(sweep.read(), 0b1000_0000);

        sweep.write(0b0010_1011);
        assert_eq!(sweep.shift, 0b011);
        assert_eq!(sweep.negate, true);
        assert_eq!(sweep.timer.period, 0b010);

        sweep.shift = 0b010;
        sweep.negate = false;
        sweep.timer.period = 0b100;

        assert_eq!(sweep.read(), 0b1100_0010);
    }
}
