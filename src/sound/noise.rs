use sound::length::Length;
use sound::envelope::Envelope;
use sound::{Sample, Timer};

pub struct NoiseChannel {
    length: Length,
    trigger_envelope: Envelope,
    envelope: Envelope,

    timer: Timer,
    lsfr: u16, // linear feedback shift register, 15 bits
    clock_shift: u8,
    lfsr_width_mode: u8,
    divisor_code: u8,

    running: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            length: Length::new(),
            trigger_envelope: Envelope::new(),
            envelope: Envelope::new(),

            timer: Timer::new(0),
            lsfr: 0,
            clock_shift: 0,
            lfsr_width_mode: 0,
            divisor_code: 0,

            running: false,
        }
    }

    pub fn tick(&mut self) {

    }

    pub fn sample(&mut self) -> Sample {
        0
    }

    pub fn tick_length(&mut self) {
        // if length runs out, turn off this channel
        if self.length.tick() {
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
        self.envelope = self.trigger_envelope;
        self.running = self.envelope.dac_enabled();

        // self.timer.period =

        self.envelope.trigger();
        self.lsfr = 0x7FFF;

    }

    // sets the envelope to be used on the next trigger
    pub fn set_envelope(&mut self, envelope: Envelope) {
        self.trigger_envelope = envelope;
    }

    pub fn get_envelope(&self) -> &Envelope {
        &self.trigger_envelope
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
