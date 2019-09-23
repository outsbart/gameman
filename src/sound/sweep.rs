use sound::{TimerDefaultPeriod};
use std::ops::{Sub, Add};


pub struct Sweep {
    pub shift: u8,
    negate: bool,           // true if calculate will operate in negate mode
    negate_mode_used: bool, // if negate mode has been used in calculate() since last trigger
    pub timer: TimerDefaultPeriod,
    shadow_frequency: u16,
    enabled: bool,
}

impl Sweep {
    pub fn new() -> Self {
        Sweep {
            shift: 0,
            negate: false,
            negate_mode_used: false,
            timer: TimerDefaultPeriod::new(),
            shadow_frequency: 0,
            enabled: false,
        }
    }

    // returns true if channel should be disabled
    // it happens when exiting negate mode after calculation
    pub fn write(&mut self, value: u8) -> bool {
        self.shift = value & 0b0000_0111;
        self.negate = value & 0b1000 != 0;
        self.timer.set_period(((value & 0b0111_0000) >> 4) as usize);

        // Clearing the sweep negate mode bit in NR10 after at least one sweep
        // calculation has been made using the negate mode since the last trigger
        // causes the channel to be immediately disabled. This prevents you from
        // having the sweep lower the frequency then raise the frequency without a
        // trigger inbetween.
        self.enabled && self.negate_mode_used && !self.negate
    }

    pub fn read(&self) -> u8 {
        0b1000_0000 |
        ((self.timer.period as u8) << 4) |
        (if self.negate {0b1000} else {0}) |
        self.shift
    }

    // return true if frequency calculations should be performed immediately
    pub fn trigger(&mut self, freq: u16) -> bool {
        // square 1's frequency is copied to the shadow register.
        self.set_shadow_frequency(freq);

        // reset negate_mode_used flag
        self.negate_mode_used = false;

        // the sweep timer is reloaded.
        self.timer.restart();

        // the internal enabled flag is set if either the sweep period or shift
        // are non-zero, cleared otherwise.
        self.enabled = (self.timer.period > 0) || (self.shift > 0);

        // if the sweep shift is non-zero, frequency calculation and the overflow
        // check are performed immediately
        self.shift > 0
    }

    // calculates the sweep, returns the new freq value and whether there was an overflow
    pub fn calculate(&mut self) -> (u16, bool) {
        // if mode is negate, we substract, otherwise we add
        let operation = if self.negate { u16::sub } else { u16::add };

        // the operands are:
        // - the shadow frequency unaltered,
        // - the shadow frequency shifted right by self.shift
        let result = operation(self.shadow_frequency, self.shadow_frequency >> self.shift as u16);

        // if we used negate mode, remember it
        if self.negate { self.negate_mode_used = true; }

        // freq is 11bit
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
