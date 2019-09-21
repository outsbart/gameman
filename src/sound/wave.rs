use sound::length::Length;
use sound::{Sample, Timer};

const WAVE_RAM_SAMPLES: u8 = 32;

pub struct WaveChannel {
    dac_power: bool,
    frequency: u16,
    length: Length,
    timer: Timer,

    position: u8,
    samples: [Sample; WAVE_RAM_SAMPLES as usize / 2],
    volume: Volume,

    // Becomes true during a trigger
    // (but is set to false if during that trigger dac is disabled or sweep overflows)
    //
    // Becomes false when one of these events happen:
    // - length counter reaches 0 and length is enabled
    // - sweep overflows
    // - dac is disabled
    running: bool,
}


#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Volume {
    Silent = 0,
    Max = 1,
    Half = 2,
    Quarter = 3,
}

impl Volume {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Volume::Silent,
            1 => Volume::Max,
            2 => Volume::Half,
            _ => Volume::Quarter,
        }
    }
}

impl Into<u8> for Volume {
    fn into(self) -> u8 {
        match self {
            Volume::Silent => 0,
            Volume::Max => 1,
            Volume::Half => 2,
            Volume::Quarter => 3,
        }
    }
}

impl Volume {
    fn apply_to(self, sample: Sample) -> Sample {
        match self {
            Volume::Silent => 0,
            Volume::Max => sample,
            Volume::Half => sample / 2,
            Volume::Quarter => sample / 4,
        }
    }
}


impl WaveChannel {
    pub fn new() -> Self {
        WaveChannel {
            dac_power: false,
            frequency: 0,
            length: Length::new(),
            timer: Timer::new(0),

            position: 0,
            samples: [0x84, 0x40, 0x43, 0xAA, 0x2D, 0x78, 0x92, 0x3C, 0x60, 0x59, 0x59, 0xB0, 0x34, 0xB8, 0x2E, 0xDA],
            volume: Volume::Silent,

            running: false,
        }
    }

    pub fn reset(&mut self) {
        self.dac_power = false;
        self.frequency = 0;
        self.length = Length::new();
        self.volume = Volume::Silent;
        self.position = 0;
    }

    pub fn tick(&mut self) {
        // ticks even if channel disabled
        if self.timer.tick() {
            self.position = self.position.wrapping_add(1) % WAVE_RAM_SAMPLES;

            // reload the timer
            self.timer.period = (2048 - self.frequency) as usize * 2;
            self.timer.restart();
        }
    }

    pub fn sample(&mut self) -> Sample {
        if !self.running || !self.dac_power { return 0 }

        let sample_byte = self.read_ram_sample(self.position / 2);

        // take first nibble if even, second if odd
        let sample = match self.position % 2 {
            0 => { sample_byte >> 4 }
            _ => { sample_byte & 0xF }
        };

        self.volume.apply_to(sample)
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn dac_enabled(&self) -> bool {
        // DAC power is controlled by the upper 5 bits of NRx2 (top bit of NR30 for
        // wave channel). If these bits are not all clear, the DAC is on, otherwise
        // it's off and outputs 0 volts.
        self.dac_power
    }

    pub fn tick_length(&mut self) {
        // if length runs out, turn off this channel
        if self.length.enabled() && self.length.tick(){
            self.running = false;
        }
    }

    pub fn trigger(&mut self) {
        self.running = true;

        // Wave channel's position is set to 0 but sample buffer is NOT refilled
        self.position = 0;

        if self.length.get_value() == 0 {
            self.length.set_value(255); // todo: make it 256
        }

        self.timer.period = (2048 - self.frequency) as usize * 2;
        self.timer.restart();

        if !self.dac_enabled() {
            self.running = false;
        }
    }

    pub fn write_ram_sample(&mut self, pos: u8, value: u8) {
        self.samples[pos as usize] = value;
    }

    pub fn read_ram_sample(&self, pos: u8) -> Sample {
        self.samples[pos as usize]
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

    pub fn write_register_0(&mut self, byte: u8) {
        self.dac_power = (byte & 0b1000_0000) != 0;

        if !self.dac_enabled() {
            self.running = false;
        }
    }

    pub fn read_register_0(&self) -> u8 {
        0b111_1111 |
        (if self.dac_power { 0b1000_0000 } else { 0 })
    }

    pub fn write_length_value(&mut self, byte: u8) {
        self.length.set_value(byte);
    }

    pub fn read_length_value(&self) -> u8 {
        self.length.get_value()
    }

    pub fn write_volume(&mut self, byte: u8) {
        self.volume = Volume::from_u8((byte & 0b0110_0000) >> 5);
    }

    pub fn read_volume(&self) -> u8 {
        0b1001_1111 |
        (self.volume as u8) << 5
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.length.set_enable(byte & 0b0100_0000 != 0);

        // set frequency most significative bits
        self.set_frequency_msb(byte);

        if byte & 0b1000_0000 != 0 {
            self.trigger();
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
    fn test_wave_dac_power() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.read_register_0(), 0b0111_1111);
        assert_eq!(channel.dac_power, false);

        channel.write_register_0(0b1000_0000);
        assert_eq!(channel.read_register_0(), 0b1111_1111);

        assert_eq!(channel.dac_power, true);

        channel.dac_power = false;
        assert_eq!(channel.read_register_0(), 0b0111_1111);
    }

    #[test]
    fn test_wave_length_load() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.length.get_value(), 0);

        channel.write_length_value(0b1110_0111);
        assert_eq!(channel.length.get_value(), 0b1110_0111);

        channel.length.set_value(0b1111_1011);
        assert_eq!(channel.read_length_value(), 0b1111_1011);
    }

    #[test]
    fn test_wave_volume() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.volume as u8, Volume::Silent as u8);
        assert_eq!(channel.read_volume(), 0b1001_1111);

        channel.write_volume(0b0110_0000);
        assert_eq!(channel.volume as u8, Volume::Quarter as u8);

        channel.volume = Volume::Max;
        assert_eq!(channel.read_volume(), 0b1011_1111);
    }


    #[test]
    fn test_wave_register_4() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.read_register_4(), 0b1011_1111);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.length.enabled(), false);
        assert_eq!(channel.frequency, 0b110_0000_0000);

        channel.length.set_enable(true);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0xFF);
    }
}
