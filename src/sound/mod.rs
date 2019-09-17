pub mod envelope;
pub mod square;
pub mod length;
pub mod wave;
pub mod noise;

use mem::Memory;

use sound::envelope::Envelope;
use sound::square::SquareChannel;
use sound::length::Length;
use sound::wave::WaveChannel;

const WAVE_TABLE_START: u16 = 0xFF30;
const SAMPLE_RATE: usize = 96000;
const BUFFER_SIZE: usize = 512;

pub type Sample = u8;


pub struct Sound {
    square_1: SquareChannel,
    square_2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,

    frame_sequencer: FrameSequencer,  // responsible for ticking the channels
    sample_timer: Timer,

    vin_l_enable: bool,
    vin_r_enable: bool,
    left_volume: u8,
    right_volume: u8,

    left_enables: ChannelsFlag,
    right_enables: ChannelsFlag,
    length_statuses: ChannelsFlag,

    // output buffer
    buffer_index: usize,
    buffer: [u8; BUFFER_SIZE],

    // sound circuit enabled?
    power: bool,
}


impl Memory for Sound {
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr & 0xff {
            0x10 => self.get_nr10(),
            0x11 => self.get_nr11(),
            0x12 => self.get_nr12(),
            0x13 => self.get_nr13(),
            0x14 => self.get_nr14(),
            0x16 => self.get_nr21(),
            0x17 => self.get_nr22(),
            0x18 => self.get_nr23(),
            0x19 => self.get_nr24(),
            0x1a => self.get_nr30(),
            0x1b => self.get_nr31(),
            0x1c => self.get_nr32(),
            0x1d => self.get_nr33(),
            0x1e => self.get_nr34(),
            0x20 => self.get_nr41(),
            0x21 => self.get_nr42(),
            0x22 => self.get_nr43(),
            0x23 => self.get_nr44(),
            0x24 => self.get_nr50(),
            0x25 => self.get_nr51(),
            0x26 => self.get_nr52(),
            0x30...0x3f => {
                self.wave.read_ram_sample((addr - WAVE_TABLE_START) as u8)
            },
            _ => 0xFF,
        }
    }

    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr & 0xff {
            0x10 => self.set_nr10(byte),
            0x11 => self.set_nr11(byte),
            0x12 => self.set_nr12(byte),
            0x13 => self.set_nr13(byte),
            0x14 => self.set_nr14(byte),
            0x16 => self.set_nr21(byte),
            0x17 => self.set_nr22(byte),
            0x18 => self.set_nr23(byte),
            0x19 => self.set_nr24(byte),
            0x1a => self.set_nr30(byte),
            0x1b => self.set_nr31(byte),
            0x1c => self.set_nr32(byte),
            0x1d => self.set_nr33(byte),
            0x1e => self.set_nr34(byte),
            0x20 => self.set_nr41(byte),
            0x21 => self.set_nr42(byte),
            0x22 => self.set_nr43(byte),
            0x23 => self.set_nr44(byte),
            0x24 => self.set_nr50(byte),
            0x25 => self.set_nr51(byte),
            0x26 => self.set_nr52(byte),
            0x30...0x3f => {
                self.wave.write_ram_sample((addr - WAVE_TABLE_START) as u8, byte);
            },
            _ => (),
        }
    }
}

pub struct ChannelsFlag {
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

            buffer_index: 0,
            buffer: [0; BUFFER_SIZE],

            power: false,
        }
    }

    pub fn tick(&mut self, t: u8) {
        for _i in 0..t {
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
//            if self.sample_timer.tick() {
//                let mut left: Sample = 0;
//                let mut right: Sample = 0;
//
//                if self.power {
//                    let s1 = self.square_1.sample();
//                    let s2 = self.square_2.sample();
//                    let s3 = self.wave.sample();
//                    let s4 = self.noise.sample();
//
//                    // mixer
//                    if self.left_enables.square_1 { left += s1 }
//                    if self.left_enables.square_2 { left += s2 }
//                    if self.left_enables.wave { left += s3 }
//                    if self.left_enables.noise { left += s4 }
//
//                    if self.right_enables.square_1 { right += s1 }
//                    if self.right_enables.square_2 { right += s2 }
//                    if self.right_enables.wave { right += s3 }
//                    if self.right_enables.noise { right += s4 }
//                }
//
//                // volume
//                left *= self.left_volume * 8;
//                right *= self.right_volume * 8;
//
//                self.output_sample(left);
//            }

        }

    }

    fn output_sample(&mut self, sample: Sample) {
        self.buffer[self.buffer_index] = sample;

        self.buffer_index += 1;

        if self.buffer_index == self.buffer.len() {
            // todo: send it to sdl2

            self.buffer_index = 0;
        }
    }

    // Square channel 1 sweep
    // NR10 FF10 -PPP NSSS Sweep period, negate, shift
    pub fn set_nr10(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_1.sweep.write(value);
    }

    pub fn get_nr10(&self) -> u8 {
        self.square_1.sweep.read()
    }

    // Square channel 1 duty and length load
    // NR11 FF11 DDLL LLLL Duty, Length load (64-L)
    pub fn set_nr11(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_1.write_register_1(value);
    }

    pub fn get_nr11(&self) -> u8 {
        self.square_1.read_register_1()
    }

    // Square channel 1 envelope
    // NR12 FF12 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr12(&mut self, value: u8) {
        if !self.power {
            return
        }
        let mut envelope = Envelope::new();
        envelope.write(value);

        self.square_1.set_envelope(envelope);
    }

    pub fn get_nr12(&self) -> u8 {
        self.square_1.get_envelope().read()
    }

    // Square channel 1 frequency LSB
    pub fn set_nr13(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_1.set_frequency_lsb(value);
    }

    pub fn get_nr13(&self) -> u8 {
        0xFF
    }

    // Square channel 1 trigger, frequency MSB and length
    pub fn set_nr14(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_1.write_register_4(value);
    }

    pub fn get_nr14(&self) -> u8 {
        self.square_1.read_register_4()
    }

    // Square channel 2 duty and length load
    // NR21 FF16 DDLL LLLL Duty, Length load (64-L)
    pub fn set_nr21(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_2.write_register_1(value);
    }

    pub fn get_nr21(&self) -> u8 {
        self.square_2.read_register_1()
    }

    // Square channel 2 envelope
    // NR22 FF17 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr22(&mut self, value: u8) {
        if !self.power {
            return
        }
        let mut envelope = Envelope::new();
        envelope.write(value);

        self.square_2.set_envelope(envelope);
    }

    pub fn get_nr22(&self) -> u8 {
        self.square_2.get_envelope().read()
    }

    // Square channel 2 frequency lsb
    // NR23 FF18 FFFF FFFF Frequency LSB
    pub fn set_nr23(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_2.set_frequency_lsb(value);
    }

    pub fn get_nr23(&self) -> u8 {
        0xFF
    }

    // Square channel 2 trigger, length and frequency msb
    // NR24 FF19 TL-- -FFF Trigger, Length enable, Frequency MSB
    pub fn set_nr24(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.square_2.write_register_4(value);
    }

    pub fn get_nr24(&self) -> u8 {
        self.square_2.read_register_4()
    }

    // Wave channel DAC power
    // NR30 FF1A E--- ---- DAC power
    pub fn set_nr30(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.wave.write_register_0(value);
    }

    pub fn get_nr30(&self) -> u8 {
        self.wave.read_register_0()
    }

    // Wave channel length load
    // NR31 FF1B LLLL LLLL Length load (256-L)
    pub fn set_nr31(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.wave.length.set_value(value)
    }

    pub fn get_nr31(&self) -> u8 {
        0xFF
    }

    // Wave channel volume
    // NR32 FF1C -VV- ---- Volume code (00=0%, 01=100%, 10=50%, 11=25%)
    pub fn set_nr32(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.wave.write_volume(value);
    }

    pub fn get_nr32(&self) -> u8 {
        self.wave.read_volume()
    }

    // Wave channel frequency lsb
    // NR33 FF1D FFFF FFFF Frequency LSB
    pub fn set_nr33(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.wave.set_frequency_lsb(value);
    }

    pub fn get_nr33(&self) -> u8 {
        0xFF
    }

    // Wave channel trigger, length, frequency MSB
    // NR34 FF1E TL-- -FFF Trigger, Length enable, Frequency MSB
    pub fn set_nr34(&mut self, value: u8) {
        if !self.power {
            return
        }
        self.wave.write_register_4(value);
    }

    pub fn get_nr34(&self) -> u8 {
        self.wave.read_register_4()
    }

    // Noise channel length load
    // NR41 FF20 --LL LLLL Length load (64-L)
    pub fn set_nr41(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.noise.length.set_value(value);
    }

    pub fn get_nr41(&self) -> u8 {
        0xFF
    }

    // Noise channel envelope
    // NR42 FF21 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr42(&mut self, value: u8) {
        if !self.power {
            return
        }

        let mut envelope = Envelope::new();
        envelope.write(value);

        self.noise.set_envelope(envelope);
    }

    pub fn get_nr42(&self) -> u8 {
        self.noise.get_envelope().read()
    }

    // Noise channel clock shift, lsfr, divisor
    // NR43 FF22 SSSS WDDD Clock shift, Width mode of LFSR, Divisor code
    pub fn set_nr43(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.noise.write_register_3(value);
    }

    pub fn get_nr43(&self) -> u8 {
        self.noise.read_register_3()
    }

    // Noise channel trigger and length enable
    // NR44 FF23 TL-- ---- Trigger, Length enable
    pub fn set_nr44(&mut self, value: u8) {
        if !self.power {
            return
        }

        self.noise.write_register_4(value);
    }

    pub fn get_nr44(&self) -> u8 {
        self.noise.read_register_4()
    }

    // NR50 FF24 ALLL BRRR	Vin L enable, Left vol, Vin R enable, Right vol
    pub fn set_nr50(&mut self, byte: u8) {
        if !self.power {
            return
        }

        self.vin_l_enable = (byte & 0b1000_0000) >> 7 != 0;
        self.vin_r_enable = (byte & 0b1000) >> 3 != 0;
        self.left_volume = (byte & 0b0111_0000) >> 4;
        self.right_volume = byte & 0b111;
}

    pub fn get_nr50(&self) -> u8 {
        (if self.vin_l_enable { 0b1000_0000 } else { 0 }) |
            (self.left_volume << 4) |
            (if self.vin_r_enable { 0b1000} else { 0 }) |
            (self.right_volume)
    }

    // NR51 FF25 NW21 NW21 Left enables, Right enables
    pub fn set_nr51(&mut self, byte: u8) {
        if !self.power {
            return
        }

        self.left_enables.write((byte & 0xF0) >> 4);
        self.right_enables.write(byte & 0xF);
    }

    pub fn get_nr51(&self) -> u8 {
        self.left_enables.read() << 4 | self.right_enables.read()
    }

    // NR52 FF26 P--- NW21 Power control/status, Channel length statuses
    pub fn set_nr52(&mut self, byte: u8) {
        self.power = byte & 0b1000_0000 != 0;
        self.length_statuses.write(byte & 0xF);

        if !self.power {
            self.reset();
        }
    }

    pub fn get_nr52(&self) -> u8 {
        0b0111_0000 |
        (if self.power { 0b1000_0000 } else { 0 })
    }

    // called when power is set to off, through register nr52
    pub fn reset(&mut self) {
        // reset all registers

        // nr 50
        self.vin_l_enable = false;
        self.vin_r_enable = false;
        self.left_volume = 0;
        self.right_volume = 0;

        // nr51
        self.left_enables.write(0);
        self.right_enables.write(0);

        // all the others
        self.square_1 = SquareChannel::new();
        self.square_2 = SquareChannel::new();

        self.wave.reset();  // wave table/ram must be left unchanged
        self.noise = NoiseChannel::new();

        // todo: reset output channels?
    }
}


pub struct FrameSequencer {
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


pub struct Timer {
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


pub struct NoiseChannel {
    length: Length,
    trigger_envelope: Envelope,
    envelope: Envelope,

    clock_shift: u8,
    lfsr_width_mode: u8,
    divisor_code: u8,

    enabled: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            length: Length::new(),
            trigger_envelope: Envelope::new(),
            envelope: Envelope::new(),

            clock_shift: 0,
            lfsr_width_mode: 0,
            divisor_code: 0,

            enabled: false,
        }
    }

    pub fn tick(&mut self) {

    }

    pub fn sample(&mut self) -> Sample {
        0
    }

    pub fn trigger(&mut self) {

    }

    // sets the envelope to be used on the next trigger
    pub fn set_envelope(&mut self, envelope: Envelope) {
        self.trigger_envelope = envelope;
        // todo: enable or disable this channel
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
    fn test_control_volume() {
        let mut sound = Sound::new();

        // enable sound
        sound.set_nr52(0x80);

        assert_eq!(sound.get_nr50(), 0);

        sound.set_nr50(0b1001_0010);
        assert_eq!(sound.vin_l_enable, true);
        assert_eq!(sound.vin_r_enable, false);
        assert_eq!(sound.left_volume, 1);
        assert_eq!(sound.right_volume, 0b10);

        sound.vin_l_enable = false;
        sound.vin_r_enable = true;
        sound.left_volume = 0b100;
        sound.right_volume = 0b111;

        assert_eq!(sound.get_nr50(), 0b0100_1111);
    }

    #[test]
    fn test_left_right_enables() {
        let mut sound = Sound::new();

        // enable sound
        sound.set_nr52(0x80);

        assert_eq!(sound.get_nr51(), 0);

        sound.set_nr51(0b1001_0010);
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
        assert_eq!(sound.get_nr51(), 0b0110_1101);
    }


    #[test]
    fn test_control_master() {
        let mut sound = Sound::new();

        assert_eq!(sound.get_nr52(), 0b0111_0000);

        sound.set_nr52(0b1000_1010);
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

        assert_eq!(sound.get_nr52(), 0b0111_0000);
    }
}
