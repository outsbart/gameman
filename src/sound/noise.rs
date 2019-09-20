use sound::length::Length;
use sound::envelope::Envelope;
use sound::{Sample, Timer};

pub struct NoiseChannel {
    length: Length,
    envelope: Envelope,

    timer: Timer,
    lfsr: u16, // linear feedback shift register, 15 bits
    clock_shift: u8,  // used to shift the divisor when restarting the clock/timer
    lfsr_width_mode: u8,
    divisor_code: u8,

    running: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            length: Length::new(),
            envelope: Envelope::new(),

            timer: Timer::new(0),
            lfsr: 0,
            clock_shift: 0,
            lfsr_width_mode: 0,
            divisor_code: 0,

            running: false,
        }
    }

    pub fn tick(&mut self) {
        if !self.timer.tick() {
            return
        }
        // When clocked by the frequency timer, the low two bits (0 and 1) are XORed, all bits are
        // shifted right by one, and the result of the XOR is put into the
        // now-empty high bit. If width mode is 1 (NR43), the XOR result is ALSO
        // put into bit 6 AFTER the shift, resulting in a 7-bit LFSR.
        let xor = (self.lfsr & 1) ^ ((self.lfsr & 0b10) >> 1);

        self.lfsr >>= 1;

        if xor != 0 {
            self.lfsr |= 0b100_0000_0000;

            if self.lfsr_width_mode != 0 {
                // todo: drop bits 15-8 or not? 7-bit LFSR or not?
                self.lfsr |= 0b100_0000
            }
        }

        self.timer.period = ((self.get_divisor() as u16) << (self.clock_shift as u16)) as usize;
        self.timer.restart();
    }

    pub fn sample(&mut self) -> Sample {
        if !self.is_running() {
            return 0;
        }

        // The waveform output is bit 0 of the LFSR, INVERTED
        if self.lfsr & 1 != 0 {
            0
        } else {
            self.envelope.get_volume()
        }
    }

    pub fn tick_length(&mut self) {
        // if length runs out, turn off this channel
        if self.length.tick() && self.length.enabled() {
            self.running = false;
        }
    }

    pub fn tick_envelope(&mut self) {
        self.envelope.tick();
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn trigger(&mut self) {
        self.running = true;

        if self.length.get_value() == 0 {
            self.length.set_value(64);
        }

        self.timer.period = ((self.get_divisor() as u16) << (self.clock_shift as u16)) as usize;
        self.timer.restart();

        self.envelope.trigger();
        self.lfsr = 0x7FFF;

        if !self.dac_enabled() {
            self.running = false;
        }
    }

    fn get_divisor(&self) -> u8 {
        match self.divisor_code {
            1 => { 16 },
            2 => { 32 },
            3 => { 48 },
            4 => { 64 },
            5 => { 80 },
            6 => { 96 },
            7 => { 112 },
            _ => { 8 }
        }
    }

    pub fn dac_enabled(&self) -> bool {
        // DAC power is controlled by the upper 5 bits of NRx2 (top bit of NR30 for
        // wave channel). If these bits are not all clear, the DAC is on, otherwise
        // it's off and outputs 0 volts.
        self.envelope.read() >> 3 != 0
    }

    // sets the envelope to be used on the next trigger
    pub fn set_envelope(&mut self, envelope: Envelope) {
        self.envelope = envelope;

        if !self.dac_enabled() {
            self.running = false;
        }
    }

    pub fn get_envelope(&self) -> &Envelope {
        &self.envelope
    }

    pub fn write_register_3(&mut self, byte: u8) {
        self.clock_shift = (byte & 0xF0) >> 4;
        self.lfsr_width_mode = (byte & 0x08) >> 3;
        self.divisor_code = byte & 0b111;
    }

    pub fn read_register_3(&self) -> u8 {
        self.clock_shift << 4 | self.lfsr_width_mode << 3 | self.divisor_code
    }

    pub fn write_length_value(&mut self, byte: u8) {
        self.length.set_value(byte);
    }

    pub fn read_length_value(&self) -> u8 {
        self.length.get_value()
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.length.set_enable(byte & 0b0100_0000 != 0);

        if byte & 0b1000_0000 != 0 {
            self.trigger()
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
    fn test_noise_register_4() {
        let mut channel: NoiseChannel = NoiseChannel::new();

        assert_eq!(channel.read_register_4(), 0b1011_1111);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.length.enabled(), false);

        channel.length.set_enable(true);

        assert_eq!(channel.read_register_4(), 0xFF);
    }

    #[test]
    fn test_noise_register_3() {
        let mut channel: NoiseChannel = NoiseChannel::new();

        assert_eq!(channel.read_register_3(), 0);

        channel.write_register_3(0b1000_1110);
        assert_eq!(channel.clock_shift, 0b1000);
        assert_eq!(channel.lfsr_width_mode, 1);
        assert_eq!(channel.divisor_code, 0b110);

        channel.clock_shift = 0b1100;
        channel.lfsr_width_mode = 0;
        channel.divisor_code = 0b1;

        assert_eq!(channel.read_register_3(), 0b1100_0001);
    }
}
