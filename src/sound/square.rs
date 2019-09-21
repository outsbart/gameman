use sound::{Length, Timer, Sample, DUTY_PATTERNS_LENGTH};
use sound::envelope::Envelope;
use cpu::is_bit_set;
use sound::length::MaxLength;

pub struct SquareChannel {
    pub sweep: Sweep,
    pub envelope: Envelope,
    pub length: Length,
    pub duty_timer: Timer,  // it resets when it runs out, and the position in the duty pattern moves forward

    duty_index: usize,  // in which position in the duty cycle we are. From 0 to 7

    // Duty Pattern
    //  0 — 00000001 (12.5%)
    //  1 — 10000001 (25.0%)
    //  2 — 10000111 (50.0%)
    //  3 — 01111110 (75.0%)
    duty: u8,
    frequency: u16,

    running: bool,
}


impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new(),  // holds the volume
            length: Length::new(MaxLength::NotWave),
            duty_timer: Timer::new(0),

            duty_index: 0,
            duty: 0,
            frequency: 0,

            // Becomes true during a trigger
            // (but is set to false if during that trigger dac is disabled or sweep overflows)
            //
            // Becomes false when one of these events happen:
            // - length counter reaches 0 and length is enabled
            // - sweep overflows
            // - dac is disabled
            running: false,
        }
    }

    pub fn tick_length(&mut self) {
        // if length runs out, turn off this channel
        // doesnt tick if it's not enabled
        if self.length.tick() {
            self.running = false;
        }
    }

    pub fn half_tick_length(&mut self) {
        self.length.half_tick();
    }

    pub fn tick_envelope(&mut self) {
        self.envelope.tick();
    }

    // the first square channel has a sweep
    pub fn tick_sweep(&mut self) {
        // The sweep timer is clocked at 128 Hz by the frame sequencer. When it
        // generates a clock and the sweep's internal enabled flag is set and the
        // sweep period is not zero, a new frequency is calculated and the overflow
        // check is performed. If the new frequency is 2047 or less and the sweep
        // shift is not zero, this new frequency is written back to the shadow
        // frequency and square 1's frequency in NR13 and NR14, then frequency
        // calculation and overflow check are run AGAIN immediately using this new
        // value, but this second new frequency is not written back.

        // timer has not run out yet
        if !self.sweep.timer.tick() {
            return
        }

        if !self.sweep.enabled() {
            return
        }

        if self.sweep.timer.period == 0 {
            return
        }

        // turns off the channel on overflow
        let (new_freq, overflow) = self.calculate_sweep();

        if overflow || self.sweep.shift == 0 {
            return
        }

        self.sweep.set_shadow_frequency(new_freq);
        self.frequency = new_freq;

        // run the freq calculation and overflows check again
        // but this time dont update the freq, just disable the channel on overflow
        self.calculate_sweep();
    }

    // calls sweep.calculate and performs the overflow check, disabling the channel if necessary
    pub fn calculate_sweep(&mut self) -> (u16, bool) {
        let (new_freq, overflow) = self.sweep.calculate();

        // overflow check
        if overflow {
            self.running = false;
        }

        (new_freq, overflow)
    }

    pub fn tick(&mut self) {
        // ticks even if channel disabled

        // timer didnt run out yet
        if !self.duty_timer.tick() {
            return
        }

        self.duty_index = (self.duty_index + 1) % DUTY_PATTERNS_LENGTH as usize;
        self.duty_timer.period = ((2048 - self.frequency) * 4) as usize;
        self.duty_timer.restart();
    }

    pub fn dac_enabled(&self) -> bool {
        // DAC power is controlled by the upper 5 bits of NRx2 (top bit of NR30 for
        // wave channel). If these bits are not all clear, the DAC is on, otherwise
        // it's off and outputs 0 volts.
        self.envelope.read() >> 3 != 0
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn sample(&mut self) -> Sample {
        if !self.is_running() {
            return 0;
        }

        let duty_pattern = self.get_duty_pattern();

        if is_bit_set((7 - self.duty_index) as u8, duty_pattern as u16) {
            return self.envelope.get_volume();
        }

        0
    }

    pub fn trigger(&mut self) {
        self.running = true;
        self.duty_index = 0;

        self.duty_timer.period = ((2048 - self.frequency) * 4) as usize;
        self.duty_timer.restart();

        // restart volume initial value and timer
        self.envelope.trigger();

        // trigger the sweep and disable the channel if it overflows
        if self.sweep.trigger(self.frequency) {
            self.calculate_sweep();
        }

        // Note that if the channel's DAC is off, after the above actions occur the
        // channel will be immediately disabled again.
        if !self.dac_enabled() {
            self.running = false;
        }
    }

    fn get_duty_pattern(&self) -> u8 {
        match self.duty {
            0 => 0b0000_0001,
            1 => 0b1000_0001,
            2 => 0b1000_0111,
            _ => 0b0111_1110,
        }
    }

    // sets the envelope for the next trigger
    pub fn set_envelope(&mut self, envelope: Envelope) {
        self.envelope = envelope;

         if !self.dac_enabled() {
             self.running = false;
         }
    }

    pub fn get_envelope(&self) -> &Envelope {
        &self.envelope
    }

    // sets frequency least significate bits
    pub fn set_frequency_lsb(&mut self, byte: u8) {
        self.frequency = (self.frequency & 0xF00) | byte as u16;
    }

    pub fn get_frequency_lsb(&self) -> u8 {
        (self.frequency & 0xFF) as u8
    }

    // sets frequency most significate bits
    pub fn set_frequency_msb(&mut self, byte: u8) {
        self.frequency = (self.frequency & 0xFF) | ((byte as u16 & 0b111) << 8);
    }

    pub fn get_frequency_msb(&self) -> u8 {
        (self.frequency >> 8) as u8
    }

    pub fn write_register_1(&mut self, byte: u8) {
        self.length.set_value(byte & 0b0011_1111);
        self.duty = (byte & 0b1100_0000) >> 6;
    }

    pub fn read_register_1(&self) -> u8 {
        (self.duty << 6) | 0b11_1111
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.set_frequency_msb(byte);

        let trigger = byte & 0b1000_0000 != 0;

        if trigger {
            self.trigger()
        }

        // enabling the length in some cases makes the length timer go down, which might reach zero
        if self.length.set_enable(byte & 0b0100_0000 != 0, trigger) {
            self.running = false;
        }
    }

    pub fn read_register_4(&self) -> u8 {
        0b1011_1111 |
        (if self.length.enabled() { 0b0100_0000 } else { 0 })
    }
}


pub struct Sweep {
    shift: u8,
    rising: bool, // true if should be increasing, false if decreasing
    pub timer: Timer,
    shadow_frequency: u16,
    enabled: bool,
}

impl Sweep {
    pub fn new() -> Self {
        Sweep {
            shift: 0,
            rising: false,
            timer: Timer::new(0),
            shadow_frequency: 0,
            enabled: false,
        }
    }

    pub fn write(&mut self, value: u8) {
        self.shift = value & 0b0000_0111;
        self.rising = value & 0b1000 != 0;
        self.timer.period = ((value & 0b0111_0000) >> 4) as usize;
    }

    pub fn read(&self) -> u8 {
        0b1000_0000 |
        ((self.timer.period as u8) << 4) |
        (if self.rising {0b1000} else {0}) |
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

        let result = if self.rising {
            self.shadow_frequency.wrapping_add(shifted)
        } else {
            self.shadow_frequency.wrapping_sub(shifted)
        };

        (result, result > 2047)
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
        assert_eq!(sweep.rising, true);
        assert_eq!(sweep.timer.period, 0b010);

        sweep.shift = 0b010;
        sweep.rising = false;
        sweep.timer.period = 0b100;

        assert_eq!(sweep.read(), 0b1100_0010);
    }

    #[test]
    fn test_square_register_1() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_1(), 0b11_1111);

        channel.write_register_1(0b1000_1111);
        assert_eq!(channel.length.get_value(), 64 - 0b1111);
        assert_eq!(channel.duty, 0b10);

        channel.length.set_value(0b1110);
        channel.duty = 0b11;

        assert_eq!(channel.read_register_1(), 0b1111_1111);
    }

    #[test]
    fn test_square_register_4() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_4(), 0b1011_1111);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.length.enabled(), false);
        assert_eq!(channel.frequency, 0b110_0000_0000);

        channel.length.set_enable(true);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0xFF);
    }
}
