use sound::{Length, Timer, Sample};
use sound::envelope::Envelope;
use cpu::is_bit_set;

pub struct SquareChannel {
    pub sweep: Sweep,
    pub envelope: Envelope,
    pub length: Length,
    pub timer: Timer,  // it resets when it runs out, and the position in the duty pattern moves forward

    pub running: bool,

    pub duty_index: usize,  // in which position in the duty cycle we are. From 0 to 7

    // Duty Pattern
    //  0 — 00000001 (12.5%)
    //  1 — 10000001 (25.0%)
    //  2 — 10000111 (50.0%)
    //  3 — 01111110 (75.0%)
    pub duty: u8,
    pub frequency: u16,

    // register 4
    pub trigger: bool,
}


impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new(),
            length: Length::new(),
            timer: Timer::new(0),

            running: false,  // is set to False from Length or Sweep

            duty_index: 0,
            duty: 0,
            frequency: 0,

            trigger: false,
        }
    }

    pub fn tick(&mut self) {
        // if timer runs out
        if self.timer.tick() {
            self.duty_index = (self.duty_index + 1) % 8;
            self.timer.curr = ((2048 - self.frequency) * 4) as usize;
        }
    }

    fn enabled(&self) -> bool {
        self.running && self.length.enabled()
    }

    pub fn sample(&mut self) -> Sample {
        if !self.running {
            return 0;
        }

        let duty_pattern = self.get_duty_pattern();

        if is_bit_set((7 - self.duty_index) as u8, duty_pattern as u16) {
            self.envelope.get_volume();
        }

        0
    }

    fn get_duty_pattern(&self) -> u8 {
        match self.duty {
            0 => 0b0000_0001,
            1 => 0b1000_0001,
            2 => 0b1000_0111,
            _ => 0b1111_1110,
        }
    }

    pub fn write_register_1(&mut self, byte: u8) {
        self.length.set_value(byte & 0b0011_1111);
        self.duty = (byte & 0b1100_0000) >> 6;
    }

    pub fn read_register_1(&self) -> u8 {
        (self.duty << 6) | self.length.get_value()
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.trigger = byte & 0b1000_0000 != 0;
        self.length.set_enable(byte & 0b0100_0000 != 0);

        // set frequency most significative bits
        self.frequency = (self.frequency & 0xFF) | ((byte as u16 & 0b111) << 8);
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length.enabled() { 0b0100_0000 } else { 0 }) |
        (self.frequency >> 8) as u8
    }
}


pub struct Sweep {
    shifts_number: u8,
    rising: bool, // true if should be increasing, false if decreasing
    time: u8,
}

impl Sweep {
    pub fn new() -> Self {
        Sweep {
            shifts_number: 0,
            rising: false,
            time: 0
        }
    }

    pub fn write(&mut self, value: u8) {
        self.shifts_number = value & 0b0000_0111;
        self.rising = value & (1 << 3) != 0;
        self.time = (value & 0b0111_0000) >> 4 ;
    }

    pub fn read(&self) -> u8 {
        (self.time << 4) |
            (if self.rising {4} else {0}) |
            self.shifts_number
    }

    pub fn tick(&self) {

    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_read_write() {
        let mut sweep: Sweep = Sweep::new();
        assert_eq!(sweep.read(), 0);

        sweep.write(0b0010_1011);
        assert_eq!(sweep.shifts_number, 0b011);
        assert_eq!(sweep.rising, true);
        assert_eq!(sweep.time, 0b010);

        sweep.shifts_number = 0b010;
        sweep.rising = false;
        sweep.time = 0b100;

        assert_eq!(sweep.read(), 0b0100_0010);
    }

    #[test]
    fn test_square_register_1() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_1(), 0);

        channel.write_register_1(0b1000_1111);
        assert_eq!(channel.length.get_value(), 0b1111);
        assert_eq!(channel.duty, 0b10);

        channel.length.set_value(0b1110);
        channel.duty = 0b11;

        assert_eq!(channel.read_register_1(), 0b1100_1110);
    }

    #[test]
    fn test_square_register_4() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_4(), 0);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.trigger, true);
        assert_eq!(channel.length.enabled(), false);
        assert_eq!(channel.frequency, 0b110_0000_0000);

        channel.trigger = false;
        channel.length.set_enable(true);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0b0100_0001);
    }
}
