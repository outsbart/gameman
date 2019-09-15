use sound::length::Length;
use sound::Sample;

pub struct WaveChannel {
    dac_power: bool,
    frequency: u16,
    pub length: Length,

    samples: [Sample; 32],
    volume: Volume,
    trigger: bool,
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

impl WaveChannel {
    pub fn new() -> Self {
        WaveChannel {
            dac_power: false,
            frequency: 0,
            length: Length::new(),

            samples: [0; 32],
            volume: Volume::Silent,
            trigger: false,
        }
    }

    pub fn tick(&mut self) {

    }

    pub fn sample(&mut self) -> Sample {
        0
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
        self.trigger = byte & 0b1000_0000 != 0;
        self.length.set_enable(byte & 0b0100_0000 != 0);

        // set frequency most significative bits
        self.set_frequency_msb(byte);
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
        assert_eq!(channel.trigger, true);
        assert_eq!(channel.length.enabled(), false);
        assert_eq!(channel.frequency, 0b110_0000_0000);

        channel.trigger = false;
        channel.length.set_enable(true);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0xFF);
    }
}
