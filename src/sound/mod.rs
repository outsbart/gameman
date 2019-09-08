use mem::Memory;


const SAMPLE_RATE: usize = 96000;


pub struct Sound {
    square_1: SquareChannel,
    square_2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,

    frame_sequencer: FrameSequencer,
    sample_timer: Timer,

    vin_l_enable: bool,
    vin_r_enable: bool,
    left_volume: u8,
    right_volume: u8,

    left_enables: ChannelsFlag,
    right_enables: ChannelsFlag,
    length_statuses: ChannelsFlag,

    power: bool,
}


impl Memory for Sound {
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr & 0xff {
            0x10 => self.square_1.sweep.read(),
            0x11 => self.square_1.read_register_1(),
            0x12 => self.square_1.envelope.read(),
            0x14 => self.square_1.read_register_4(),
            0x16 => self.square_2.read_register_1(),
            0x17 => self.square_2.envelope.read(),
            0x19 => self.square_2.read_register_4(),
            0x1a => self.wave.read_register_0(),
            0x1c => self.wave.read_volume(),
            0x1e => self.wave.read_register_4(),
            0x21 => self.noise.envelope.read(),
            0x22 => self.noise.read_register_3(),
            0x23 => self.noise.read_register_4(),
            0x24 => self.read_control_volume(),
            0x25 => self.read_channel_enables(),
            0x26 => self.read_control_master(),
            0x30...0x3f => { panic!("wave channel ram not implemented") },
            _ => 0xff,
        }
    }

    fn write_byte(&mut self, addr: u16, byte: u8) {

    }
}

struct ChannelsFlag {
    noise: bool,
    wave: bool,
    square_2: bool,
    square_1: bool,
}

impl ChannelsFlag {
    pub fn new() -> Self {
        ChannelsFlag {
            noise: false,
            wave: false,
            square_2: false,
            square_1: false,
        }
    }

    pub fn write(&mut self, byte: u8) {
        self.noise = (byte & 0b1000) >> 3 != 0;
        self.wave = (byte & 0b100) >> 2 != 0;
        self.square_2 = (byte & 0b10) >> 1 != 0;
        self.square_1 = byte & 0b1 != 0;
    }

    pub fn read(&self) -> u8 {
        (if self.noise { 0b1000 } else { 0 }) |
        (if self.wave { 0b100 } else { 0 }) |
        (if self.square_2 { 0b10 } else { 0 }) |
        (if self.square_1 { 1 } else { 0 })
    }
}

impl Sound {
    pub fn new() -> Self {
        Sound {
            square_1: SquareChannel::new(),
            square_2: SquareChannel::new(),
            wave: WaveChannel::new(),
            noise: NoiseChannel::new(),

            frame_sequencer: FrameSequencer::new(),
            sample_timer: Timer::new(4194304 / SAMPLE_RATE),

            vin_l_enable: false,
            vin_r_enable: false,

            left_volume: 0,
            right_volume: 0,

            left_enables: ChannelsFlag::new(),
            right_enables: ChannelsFlag::new(),
            length_statuses: ChannelsFlag::new(),

            power: false
        }
    }


    pub fn read_control_volume(&self) -> u8 {
        (if self.vin_l_enable { 0b1000_0000 } else { 0 }) |
            (self.left_volume << 4) |
            (if self.vin_r_enable { 0b1000} else { 0 }) |
            (self.right_volume)
    }

    pub fn write_control_volume(&mut self, byte: u8) {
        self.vin_l_enable = (byte & 0b1000_0000) >> 7 != 0;
        self.vin_r_enable = (byte & 0b1000) >> 3 != 0;
        self.left_volume = (byte & 0b0111_0000) >> 4;
        self.right_volume = byte & 0b111;
    }

    pub fn read_channel_enables(&self) -> u8 {
        self.left_enables.read() << 4 | self.right_enables.read()
    }

    pub fn write_channel_enables(&mut self, byte: u8) {
        self.left_enables.write((byte & 0xF0) >> 4);
        self.right_enables.write(byte & 0xF);
    }

    pub fn read_control_master(&self) -> u8 {
        (if self.power { 0b1000_0000 } else { 0 }) |
            self.length_statuses.read()
    }

    pub fn write_control_master(&mut self, byte: u8) {
        self.power = byte & 0b1000_0000 != 0;
        self.length_statuses.write(byte & 0xF);
    }


    pub fn tick(&mut self) {
        for _i in 0u8..4 {
            self.square_1.tick();
            self.square_2.tick();
            self.wave.tick();
            self.noise.tick();

            // if sequence timer has finished/reached zero
            if self.frame_sequencer.tick() {

                // every 2 steps we tick the channel length counters
                if self.frame_sequencer.step % 2 == 0 {
                    self.square_1.length.tick();
                    self.square_2.length.tick();
                    self.wave.length.tick();
                    self.noise.length.tick();
                }

                // at step 7, tick the channel envelopes
                if self.frame_sequencer.step == 7 {
                    self.square_1.envelope.tick();
                    self.square_2.envelope.tick();
                    self.noise.envelope.tick();
                }

                // at step 2 and 6 tick the sweep
                if self.frame_sequencer.step == 2 || self.frame_sequencer.step == 6 {
                    self.square_1.sweep.tick();
                }
            }

            // fetch the samples!
        }
    }
}


struct FrameSequencer {
    timer: Timer,
    step: u8,      // goes up by 1 everytime the timer hits 0
    step_max: u8,  // indicates at which value step should go back to 0
}

impl FrameSequencer {
    pub fn new() -> Self {
        FrameSequencer {
            timer: Timer::new(8192),
            step: 0,
            step_max: 8,
        }
    }

    // ticks the timer and increases step when the timer hits 0
    pub fn tick(&mut self) -> bool {
        let timer_up = self.timer.tick();
        if timer_up {
            self.step = (self.step + 1) % self.step_max;
        }
        timer_up
    }
}


struct Timer {
    period: usize, // initial and max value of curr
    curr: usize,   // goes down by 1 every tick and wraps back to period
}


impl Timer {
    pub fn new(period: usize) -> Self {
        Timer {
            period,
            curr: period,
        }
    }

    // returns true when the timer hits 0
    pub fn tick(&mut self) -> bool {
        if self.curr == 0 {
            self.curr = self.period;
            return true;
        }

        self.curr = self.curr.wrapping_sub(1);
        false
    }
}


// every tick, adjusts volume
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

    pub fn tick(&mut self) {

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

    pub fn tick(&self) {

    }
}



// used to shut off a channel after a period of time
struct Length {
    value: u8,
    enable: bool,
}

impl Length {
    pub fn new() -> Self {
        Length {
            value: 0,
            enable: false,
        }
    }

    pub fn tick(&mut self) {

    }
}


struct SquareChannel {
    sweep: Sweep,
    envelope: Envelope,
    length: Length,

    // register 1
    duty: u8,
    frequency: u8,

    // register 4
    trigger: bool,
    frequency_msb: u8,
}

impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new(),
            length: Length::new(),

            duty: 0,
            frequency: 0,

            trigger: false,
            frequency_msb: 0
        }
    }

    pub fn tick(&mut self) {

    }

    pub fn write_register_1(&mut self, byte: u8) {
        self.length.value = byte & 0b0011_1111;
        self.duty = (byte & 0b1100_0000) >> 6;
    }

    pub fn read_register_1(&self) -> u8 {
        (self.duty << 6) | self.length.value
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.trigger = byte & 0b1000_0000 != 0;
        self.length.enable = byte & 0b0100_0000 != 0;
        self.frequency_msb = byte & 0b111;
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length.enable { 0b0100_0000 } else { 0 }) |
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
    frequency: u8,
    length: Length,

    volume: Volume,
    trigger: bool,
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
            frequency: 0,
            length: Length::new(),

            volume: Volume::Silent,
            trigger: false,
            frequency_msb: 0
        }
    }

    pub fn tick(&mut self) {

    }

    pub fn write_register_0(&mut self, byte: u8) {
        self.dac_power = (byte & 0b1000_0000) != 0;
    }

    pub fn read_register_0(&self) -> u8 {
        if self.dac_power { 0b1000_0000 } else { 0 }
    }

    pub fn write_length_value(&mut self, byte: u8) {
        self.length.value = byte;
    }

    pub fn read_length_value(&self) -> u8 {
        self.length.value
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
        self.length.enable = byte & 0b0100_0000 != 0;
        self.frequency_msb = byte & 0b111;
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length.enable { 0b0100_0000 } else { 0 }) |
        self.frequency_msb
    }
}

struct NoiseChannel {
    length: Length,
    envelope: Envelope,

    clock_shift: u8,
    lfsr_width_mode: u8,
    divisor_code: u8,

    trigger: bool,

}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            length: Length::new(),
            envelope: Envelope::new(),

            clock_shift: 0,
            lfsr_width_mode: 0,
            divisor_code: 0,

            trigger: false,
        }
    }

    pub fn tick(&mut self) {

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
        self.length.value = byte;
    }

    pub fn read_length_value(&self) -> u8 {
        self.length.value
    }

    pub fn write_register_4(&mut self, byte: u8) {
        self.trigger = byte & 0b1000_0000 != 0;
        self.length.enable = byte & 0b0100_0000 != 0;
    }

    pub fn read_register_4(&self) -> u8 {
        (if self.trigger { 0b1000_0000 } else { 0 }) |
        (if self.length.enable { 0b0100_0000 } else { 0 })
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
        assert_eq!(channel.length.value, 0b1111);
        assert_eq!(channel.duty, 0b10);

        channel.length.value = 0b1110;
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
        assert_eq!(channel.length.enable, false);
        assert_eq!(channel.frequency_msb, 0b110);

        channel.trigger = false;
        channel.length.enable = true;
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

        channel.write_register_0(0b1110_0111);
        assert_eq!(channel.dac_power, true);

        channel.dac_power = false;
        assert_eq!(channel.read_register_0(), 0b0000_0000);
    }

    #[test]
    fn test_wave_length_load() {
        let mut channel: WaveChannel = WaveChannel::new();

        assert_eq!(channel.length.value, 0);

        channel.write_length_value(0b1110_0111);
        assert_eq!(channel.length.value, 0b1110_0111);

        channel.length.value = 0b1111_1011;
        assert_eq!(channel.read_length_value(), 0b1111_1011);
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
        assert_eq!(channel.length.enable, false);
        assert_eq!(channel.frequency_msb, 0b110);

        channel.trigger = false;
        channel.length.enable = true;
        channel.frequency_msb = 0b001;

        assert_eq!(channel.read_register_4(), 0b0100_0001);
    }

    #[test]
    fn test_noise_register_4() {
        let mut channel: NoiseChannel = NoiseChannel::new();

        assert_eq!(channel.read_register_4(), 0);

        channel.write_register_4(0b1000_1110);
        assert_eq!(channel.trigger, true);
        assert_eq!(channel.length.enable, false);

        channel.trigger = false;
        channel.length.enable = true;

        assert_eq!(channel.read_register_4(), 0b0100_0000);
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

    #[test]
    fn test_control_volume() {
        let mut sound = Sound::new();

        assert_eq!(sound.read_control_volume(), 0);

        sound.write_control_volume(0b1001_0010);
        assert_eq!(sound.vin_l_enable, true);
        assert_eq!(sound.vin_r_enable, false);
        assert_eq!(sound.left_volume, 1);
        assert_eq!(sound.right_volume, 0b10);

        sound.vin_l_enable = false;
        sound.vin_r_enable = true;
        sound.left_volume = 0b100;
        sound.right_volume = 0b111;

        assert_eq!(sound.read_control_volume(), 0b0100_1111);
    }

    #[test]
    fn test_left_right_enables() {
        let mut sound = Sound::new();

        assert_eq!(sound.read_channel_enables(), 0);

        sound.write_channel_enables(0b1001_0010);
        assert_eq!(sound.left_enables.noise, true);
        assert_eq!(sound.left_enables.wave, false);
        assert_eq!(sound.left_enables.square_2, false);
        assert_eq!(sound.left_enables.square_1, true);
        assert_eq!(sound.right_enables.noise, false);
        assert_eq!(sound.right_enables.wave, false);
        assert_eq!(sound.right_enables.square_2, true);
        assert_eq!(sound.right_enables.square_1, false);

        sound.left_enables.noise = false;
        sound.left_enables.wave = true;
        sound.left_enables.square_2 = true;
        sound.left_enables.square_1 = false;
        sound.right_enables.noise = true;
        sound.right_enables.wave = true;
        sound.right_enables.square_2 = false;
        sound.right_enables.square_1 = true;
        assert_eq!(sound.read_channel_enables(), 0b0110_1101);
    }


    #[test]
    fn test_control_master() {
        let mut sound = Sound::new();

        assert_eq!(sound.read_control_master(), 0);

        sound.write_control_master(0b1000_1010);
        assert_eq!(sound.power, true);
        assert_eq!(sound.length_statuses.noise, true);
        assert_eq!(sound.length_statuses.wave, false);
        assert_eq!(sound.length_statuses.square_2, true);
        assert_eq!(sound.length_statuses.square_1, false);

        sound.power = false;
        sound.length_statuses.noise = false;
        sound.length_statuses.wave = true;
        sound.length_statuses.square_2 = false;
        sound.length_statuses.square_1 = true;

        assert_eq!(sound.read_control_master(), 0b0000_0101);
    }
}
