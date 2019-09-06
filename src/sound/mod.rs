use mem::Memory;

pub struct Sound {
    channel_1: SquareChannel,
    channel_2: SquareChannel,
    channel_3: WaveChannel,
    channel_4: NoiseChannel
}


impl Memory for Sound {
    fn read_byte(&mut self, addr: u16) -> u8 {
//        match addr & 0xff {
//            0x10 => self.channel_1.sweep.read(),
//            0x11 => self.channel_1.read_reg1(),
//            0x12 => self.channel_1.envelope.read_reg(),
//            0x14 => self.channel_1.read_reg4(),
//            0x16 => self.channel_2.read_reg1(),
//            0x17 => self.channel_2.envelope.read_reg(),
//            0x19 => self.channel_2.read_reg4(),
//            0x1a => self.channel_3.read_reg0(),
//            0x1c => self.channel_3.read_reg2(),
//            0x1e => self.channel_3.read_reg4(),
//            0x21 => self.channel_4.envelope.read_reg(),
//            0x22 => self.channel_4.read_reg3(),
//            0x23 => self.channel_4.read_reg4(),
//            0x24 => self.get_ctrl_volume(),
//            0x25 => self.get_terminal_channels(),
//            0x26 => self.get_ctrl_master(),
//            0x30...0x3f => self.channel_3.read_wave_ram(addr - 0xff30),
//            _ => 0xff,
//        }
        0
    }

    fn write_byte(&mut self, addr: u16, byte: u8) {

    }
}

impl Sound {
    pub fn new() -> Self {
        Sound {
            channel_1: SquareChannel::new(),
            channel_2: SquareChannel::new(),
            channel_3: WaveChannel::new(),
            channel_4: NoiseChannel::new()
        }
    }
}


struct Envelope {
    start_volume: u8,
    add_mode: bool,
    period: u8
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            start_volume: 0,
            add_mode: false,
            period: 0,
        }
    }

    pub fn write(&mut self, byte: u8) {
        self.period = byte & 0b111;
        self.add_mode = (byte & 0b1000) >> 3 != 0;
        self.start_volume = (byte & 0xF0) >> 4;
    }

    pub fn read(&self) -> u8 {
        self.period | (if self.add_mode == true { 0b1000 } else { 0 }) | (self.start_volume << 4)
    }
}


struct Sweep {
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
}


struct SquareChannel {
    pub sweep: Sweep,
    pub envelope: Envelope,

    // register 1
    pub duty: u8,
    pub length_load: u8,
    pub frequency: u8,

    // register 4
    trigger: bool,
    length_enable: bool,
    frequency_msb: u8,
}

impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new(),

            duty: 0,
            length_load: 0,
            frequency: 0,

            trigger: false,
            length_enable: false,
            frequency_msb: 0
        }
    }

    pub fn write_register_1(&mut self, byte: u8) {
        self.length_load = byte & 0b0011_1111;
        self.duty = (byte & 0b1100_0000) >> 6;
    }

    pub fn read_register_1(&self) -> u8 {
        (self.duty << 6) | self.length_load
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.trigger = byte & 0b1000_0000 != 0;
        self.length_enable = byte & 0b0100_0000 != 0;
        self.frequency_msb = byte & 0b111;
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length_enable { 0b0100_0000 } else { 0 }) |
        self.frequency_msb
    }

    pub fn write_frequency(&mut self, byte: u8) {
        self.frequency = byte;
    }

    pub fn read_frequency(&self) -> u8 {
        self.frequency
    }
}

struct WaveChannel {
    dac_power: bool,
    length_load: u8,
    frequency: u8,

    volume: Volume,
    trigger: bool,
    length_enable: bool,
    frequency_msb: u8,
}


#[derive(Clone, Copy)]
#[repr(u8)]
enum Volume {
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
            length_load: 0,
            frequency: 0,

            volume: Volume::Silent,
            trigger: false,
            length_enable: false,
            frequency_msb: 0
        }
    }

    pub fn write_dac_power(&mut self, byte: u8) {
        self.dac_power = (byte & 0b1000_0000) != 0;
    }

    pub fn read_dac_power(&self) -> u8 {
        if self.dac_power { 0b1000_0000 } else { 0 }
    }

    pub fn write_length_load(&mut self, byte: u8) {
        self.length_load = byte;
    }

    pub fn read_length_load(&self) -> u8 {
        self.length_load
    }

    pub fn write_frequency(&mut self, byte: u8) {
        self.frequency = byte;
    }

    pub fn read_frequency(&self) -> u8 {
        self.frequency
    }

    pub fn write_volume(&mut self, byte: u8) {
        self.volume = Volume::from_u8((byte & 0b0110_0000) >> 5);
    }

    pub fn read_volume(&self) -> u8 {
        (self.volume as u8) << 5
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.trigger = byte & 0b1000_0000 != 0;
        self.length_enable = byte & 0b0100_0000 != 0;
        self.frequency_msb = byte & 0b111;
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length_enable { 0b0100_0000 } else { 0 }) |
        self.frequency_msb
    }
}

struct NoiseChannel {

}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel { }
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
        assert_eq!(channel.length_load, 0b1111);
        assert_eq!(channel.duty, 0b10);

        channel.length_load = 0b1110;
        channel.duty = 0b11;

        assert_eq!(channel.read_register_1(), 0b1100_1110);
    }

    #[test]
    fn test_envelope() {
        let mut envelope: Envelope = Envelope::new();

        assert_eq!(envelope.read(), 0);

        envelope.write(0b1000_1011);
        assert_eq!(envelope.period, 0b011);
        assert_eq!(envelope.add_mode, true);
        assert_eq!(envelope.start_volume, 0b1000);

        envelope.start_volume = 0b1110;
        envelope.add_mode = false;
        envelope.period = 0b111;

        assert_eq!(envelope.read(), 0b1110_0111);
    }

    #[test]
    fn test_square_frequency() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.frequency, 0);

        channel.write_frequency(0b1110_0111);
        assert_eq!(channel.frequency, 0b1110_0111);

        channel.frequency = 0b1111_1011;
        assert_eq!(channel.read_frequency(), 0b1111_1011);
    }

    #[test]
    fn test_square_register_4() {
        let mut channel: SquareChannel = SquareChannel::new();

        assert_eq!(channel.read_register_4(), 0);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.trigger, true);
        assert_eq!(channel.length_enable, false);
        assert_eq!(channel.frequency_msb, 0b110);

        channel.trigger = false;
        channel.length_enable = true;
        channel.frequency_msb = 0b001;

        assert_eq!(channel.read_register_4(), 0b0100_0001);
    }

    #[test]
    fn test_wave_frequency() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.frequency, 0);

        channel.write_frequency(0b1110_0111);
        assert_eq!(channel.frequency, 0b1110_0111);

        channel.frequency = 0b1111_1011;
        assert_eq!(channel.read_frequency(), 0b1111_1011);
    }

    #[test]
    fn test_wave_dac_power() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.dac_power, false);

        channel.write_dac_power(0b1110_0111);
        assert_eq!(channel.dac_power, true);

        channel.dac_power = false;
        assert_eq!(channel.read_dac_power(), 0b0000_0000);
    }

    #[test]
    fn test_wave_length_load() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.length_load, 0);

        channel.write_length_load(0b1110_0111);
        assert_eq!(channel.length_load, 0b1110_0111);

        channel.length_load = 0b1111_1011;
        assert_eq!(channel.read_length_load(), 0b1111_1011);
    }

    #[test]
    fn test_wave_volume() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.volume as u8, Volume::Silent as u8);

        channel.write_volume(0b0110_0000);
        assert_eq!(channel.volume as u8, Volume::Quarter as u8);

        channel.volume = Volume::Max;
        assert_eq!(channel.read_volume(), 0b0010_0000);
    }


    #[test]
    fn test_wave_register_4() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.read_register_4(), 0);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.trigger, true);
        assert_eq!(channel.length_enable, false);
        assert_eq!(channel.frequency_msb, 0b110);

        channel.trigger = false;
        channel.length_enable = true;
        channel.frequency_msb = 0b001;

        assert_eq!(channel.read_register_4(), 0b0100_0001);
    }
}
