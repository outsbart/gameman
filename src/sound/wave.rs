use sound::length::{Length, MaxLength};
use sound::{Sample, Timer, Voltage};

const WAVE_RAM_SAMPLES: u8 = 32;

pub struct WaveChannel {
    dac_power: bool,
    frequency: u16,
    length: Length,
    timer: Timer,

    wave_ram_accessible: bool, // if channel is enabled, wave ram can be accessed from outside only when accessed by the wave channel recently
    pub buffer: u8,
    pub position: u8,
    samples: [u8; WAVE_RAM_SAMPLES as usize / 2],
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

impl From<u8> for Volume {
    fn from(val: u8) -> Self {
        match val {
            0 => Volume::Silent,
            1 => Volume::Max,
            2 => Volume::Half,
            3 => Volume::Quarter,
            _ => panic!("Impossible volume"),
        }
    }
}

impl Volume {
    fn apply_to(self, sample: Sample) -> Sample {
        Sample(match self {
            Volume::Silent => 0,
            Volume::Max => sample.0,
            Volume::Half => sample.0 / 2,
            Volume::Quarter => sample.0 / 4,
        })
    }
}

impl WaveChannel {
    pub fn new() -> Self {
        WaveChannel {
            dac_power: false,
            frequency: 0,
            length: Length::new(MaxLength::Wave),
            timer: Timer::new(0),

            wave_ram_accessible: false,
            buffer: 0,
            position: 0,
            samples: [
                0x84, 0x40, 0x43, 0xAA, 0x2D, 0x78, 0x92, 0x3C, 0x60, 0x59, 0x59, 0xB0, 0x34, 0xB8,
                0x2E, 0xDA,
            ],
            volume: Volume::Silent,

            running: false,
        }
    }

    pub fn reset(&mut self) {
        // wave table/ram must be left unchanged
        self.position = 0;
        self.timer = Timer::new(0);
        self.running = false;
        self.wave_ram_accessible = false;
    }

    pub fn tick(&mut self) {
        // ticks even if channel disabled
        if self.timer.tick() {
            self.position = (self.position + 1) % WAVE_RAM_SAMPLES;

            self.buffer = self.samples[self.position as usize / 2];
            self.wave_ram_accessible = true;

            // reload the timer
            self.timer.period = (2048 - self.frequency) as usize * 2;
            self.timer.restart();
        } else {
            self.wave_ram_accessible = false;
        }
    }

    fn sample(&mut self) -> Sample {
        if !self.is_running() || !self.dac_enabled() {
            return Sample(0);
        }

        // take first nibble if even, second if odd
        let sample = Sample(match self.position % 2 {
            0 => self.buffer >> 4,
            _ => self.buffer & 0xF,
        });

        self.volume.apply_to(sample)
    }

    pub fn output(&mut self) -> Voltage {
        self.sample().to_voltage()
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
        // doesnt tick if it's not enabled
        if self.length.tick() {
            self.running = false;
        }
    }

    pub fn half_tick_length(&mut self) {
        self.length.half_tick();
    }

    pub fn trigger(&mut self) {
        let was_enabled = self.running;

        self.running = true;

        if !self.dac_enabled() {
            self.running = false;
        } else if was_enabled && self.timer.curr <= 2 {
            // Only on DMG
            // Triggering the wave channel on the DMG while it reads a sample byte
            // will alter the first four bytes of wave RAM
            self.corrupt_wave();
        }

        // Wave channel's position is set to 0 but sample buffer is NOT refilled
        self.position = 0;

        self.timer.period = (2048 - self.frequency) as usize * 2 + 6;
        self.timer.restart();
    }

    pub fn write_ram_sample(&mut self, pos: u8, value: u8) {
        // If the wave channel is enabled, accessing any byte from $FF30-$FF3F is
        // equivalent to accessing the current byte selected by the waveform
        // position. Further, on the DMG accesses will only work in this manner if
        // made within a couple of clocks of the wave channel accessing wave RAM;
        // if made at any other time, reads return $FF and writes have no effect.
        if !self.running {
            self.samples[pos as usize] = value;
            return;
        }
        if self.wave_ram_accessible {
            self.samples[self.position as usize / 2] = value;
        }
    }

    pub fn read_ram_sample(&self, pos: u8) -> u8 {
        // Just like write
        if !self.running {
            return self.samples[pos as usize];
        }
        if self.wave_ram_accessible {
            return self.samples[self.position as usize / 2];
        }
        0xFF
    }

    fn corrupt_wave(&mut self) {
        // If the channel was reading
        // one of the first four bytes, only the first byte will be rewritten with
        // the byte being read. If the channel was reading one of the later 12
        // bytes, the first FOUR bytes of wave RAM will be rewritten with the four
        // aligned bytes that the read was from (bytes 4-7, 8-11, or 12-15); for
        // example if it were reading byte 9 when it was retriggered, the first
        // four bytes would be rewritten with the contents of bytes 8-11

        // We are interested in the sample that will be picked next
        let next_sample_position = (self.position + 1) % WAVE_RAM_SAMPLES;
        let byte_position = next_sample_position as usize / 2;

        // 0 indicates bytes 0-3, 1 indicates bytes 4-7, ... i indicates [4*i]-[4*i+3]
        let quartet_index = byte_position / 4;

        if quartet_index == 0 {
            self.samples[0] = self.samples[byte_position]
        } else {
            for j in 0..4 {
                self.samples[j] = self.samples[4 * quartet_index + j]
            }
        }
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
        0b111_1111 | (if self.dac_power { 0b1000_0000 } else { 0 })
    }

    pub fn write_length_value(&mut self, byte: u8) {
        self.length.set_value(byte);
    }

    pub fn read_length_value(&self) -> u16 {
        self.length.get_value()
    }

    pub fn write_volume(&mut self, byte: u8) {
        self.volume = Volume::from_u8((byte & 0b0110_0000) >> 5);
    }

    pub fn read_volume(&self) -> u8 {
        0b1001_1111 | (self.volume as u8) << 5
    }

    pub fn write_register_4(&mut self, byte: u8) {
        // set frequency most significative bits
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
        0b1011_1111
            | (if self.length.enabled() {
                0b0100_0000
            } else {
                0
            })
    }
}

impl Default for WaveChannel {
    fn default() -> Self {
        WaveChannel::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_dac_power() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.read_register_0(), 0b0111_1111);
        assert!(!channel.dac_power);

        channel.write_register_0(0b1000_0000);
        assert_eq!(channel.read_register_0(), 0b1111_1111);

        assert!(channel.dac_power);

        channel.dac_power = false;
        assert_eq!(channel.read_register_0(), 0b0111_1111);
    }

    #[test]
    fn test_wave_length_load() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.length.get_value(), 0);

        channel.write_length_value(0b1110_0111);
        assert_eq!(channel.length.get_value(), 256 - 0b1110_0111);

        channel.length.set_value(0b1111_1011);
        assert_eq!(channel.read_length_value(), 256 - 0b1111_1011);
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
        assert!(!channel.length.enabled());
        assert_eq!(channel.frequency, 0b110_0000_0000);

        channel.length.set_enable(true, false);
        channel.frequency = 0b001_0000_0000;

        assert_eq!(channel.read_register_4(), 0xFF);
    }
}
