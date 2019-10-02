use cpu::is_bit_set;
use sound::{DUTY_PATTERNS_LENGTH, Length, Sample, Timer, Voltage};
use sound::envelope::Envelope;
use sound::length::MaxLength;
use sound::sweep::Sweep;

pub struct SquareChannel {
    sweep: Sweep,
    pub envelope: Envelope,
    pub length: Length,
    pub duty_timer: Timer,  // it resets when it runs out, and the position in the duty pattern moves forward

    pub duty_index: usize,  // in which position in the duty cycle we are. From 0 to 7

    // Duty Pattern
    //  0 — 00000001 (12.5%)
    //  1 — 10000001 (25.0%)
    //  2 — 10000111 (50.0%)
    //  3 — 01111110 (75.0%)
    duty: u8,
    frequency: u16,  // it's 11 bits

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
            // - reset
            running: false,
        }
    }

    pub fn tick_length(&mut self) {
        // if length runs out, turn off this channel
        // internally, doesnt tick if it's not enabled
        if self.length.tick() {
            self.running = false;
        }
    }

    pub fn half_tick_length(&mut self) {
        self.length.half_tick();
    }

    pub fn tick_envelope(&mut self) {
        if !self.is_running() {
            return;
        }

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

        if !overflow && self.sweep.shift != 0 {
            self.sweep.set_shadow_frequency(new_freq);
            self.frequency = new_freq;
        }

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

    fn sample(&mut self) -> Sample {
        if !self.is_running() || !self.dac_enabled() { return Sample(0) }

        let duty_pattern = self.get_duty_pattern();

        if is_bit_set((7 - self.duty_index) as u8, duty_pattern as u16) {
            return self.envelope.get_volume();
        }

        Sample(0)
    }

    pub fn output(&mut self) -> Voltage {
        self.sample().to_voltage()
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.duty_timer = Timer::new(0);
        self.duty_index = 0;
    }

    pub fn write_sweep(&mut self, byte: u8) {
        if self.sweep.write(byte) {
            self.running = false;
        }
    }

    pub fn read_sweep(&self) -> u8 {
        self.sweep.read()
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_register_1() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_1(), 0b11_1111);

        channel.write_register_1(0b1000_1111);
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

        channel.length.set_enable(true, false);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0xFF);
    }
}
