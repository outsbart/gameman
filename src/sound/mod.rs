use std::ops::{Add, AddAssign};

use cpu::CPU_FREQ;
use mem::Memory;
use sound::envelope::Envelope;
use sound::length::Length;
use sound::noise::NoiseChannel;
use sound::square::SquareChannel;
use sound::wave::WaveChannel;

pub mod envelope;
pub mod length;
pub mod noise;
pub mod square;
pub mod sweep;
pub mod wave;

pub const AUDIO_BUFFER_SIZE: usize = 1024;
pub const SAMPLE_RATE: usize = 44_100;

const WAVE_TABLE_START: u16 = 0xFF30;
const DUTY_PATTERNS_LENGTH: u8 = 8;

// final volume is moltiplied by this value
const VOLUME_BOOST: u8 = 3;

type AudioOutType = i16;

#[derive(Eq, Clone, Copy)]
pub struct Sample(u8);
const SAMPLE_MAX: Sample = Sample(0xF);
const SAMPLE_MIN: Sample = Sample(0);

impl Sample {
    fn increase(&mut self) {
        if self.0 < SAMPLE_MAX.0 {
            *self = Sample(self.0 + 1)
        }
    }

    fn decrease(&mut self) {
        if self.0 > SAMPLE_MIN.0 {
            *self = Sample(self.0 - 1)
        }
    }
}

impl PartialEq for Sample {
    fn eq(&self, b: &Sample) -> bool {
        self.0 == b.0
    }
}

impl From<Sample> for u8 {
    fn from(sample: Sample) -> Self {
        sample.0
    }
}

impl Sample {
    fn to_voltage(self) -> Voltage {
        Voltage::from(self)
    }
}

#[derive(Clone, Copy)]
pub struct Voltage(i16);

impl Add for Voltage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Voltage(self.0 + rhs.0)
    }
}

impl AddAssign for Voltage {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs)
    }
}

impl Voltage {
    // converts the voltage in the desired output type
    fn to_out_type(self) -> AudioOutType {
        self.0
    }
}

impl From<Sample> for Voltage {
    // this converts the input value to a proportional output voltage. An input of 0
    // generates -1.0 and an input of 15 generates +1.0, using arbitrary
    // voltage units.
    fn from(sample: Sample) -> Self {
        Voltage(u8::from(SAMPLE_MAX) as i16 - (u8::from(sample) as i16 * 2))
    }
}

pub struct Sound {
    square_1: SquareChannel,
    square_2: SquareChannel,
    wave: WaveChannel,
    noise: NoiseChannel,

    frame_sequencer: FrameSequencer, // responsible for ticking the channels
    sample_timer: Timer,             // timer for fetching the channels output

    left_sound_output: SoundOutput,
    right_sound_output: SoundOutput,

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
            0x30..=0x3f => self.wave.read_ram_sample((addr - WAVE_TABLE_START) as u8),
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
            0x30..=0x3F => {
                self.wave
                    .write_ram_sample((addr - WAVE_TABLE_START) as u8, byte);
            }
            _ => (),
        }
    }
}

pub struct ChannelsOutput {
    square_1: Voltage,
    square_2: Voltage,
    wave: Voltage,
    noise: Voltage,
}

impl ChannelsOutput {
    pub fn new() -> Self {
        ChannelsOutput {
            square_1: Voltage(0),
            square_2: Voltage(0),
            wave: Voltage(0),
            noise: Voltage(0),
        }
    }
}

impl Default for ChannelsOutput {
    fn default() -> Self {
        ChannelsOutput::new()
    }
}

struct SoundOutput {
    mixer: Mixer,
    volume_master: VolumeMaster,
    out_buffer: OutputBuffer,
}

impl SoundOutput {
    pub fn new() -> Self {
        SoundOutput {
            mixer: Mixer::new(),
            volume_master: VolumeMaster::new(),
            out_buffer: OutputBuffer::new(),
        }
    }

    pub fn receive(&mut self, channel_outputs: ChannelsOutput) {
        let mixed = self.mixer.mix(channel_outputs);
        let scaled = self.volume_master.apply(mixed);

        self.out_buffer.push(scaled);
    }
}

pub struct VolumeMaster {
    volume: u8,
}

impl VolumeMaster {
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    pub fn apply(&self, voltage: Voltage) -> Voltage {
        Voltage(voltage.0 * (self.volume + 1) as i16)
    }

    pub fn new() -> Self {
        VolumeMaster { volume: 0 }
    }
}

impl Default for VolumeMaster {
    fn default() -> Self {
        VolumeMaster::new()
    }
}

// Mixes together the sound voltages from the channels
pub struct Mixer {
    noise: bool,
    wave: bool,
    square_2: bool,
    square_1: bool,
    vin: bool,
}

impl Mixer {
    pub fn new() -> Self {
        Mixer {
            noise: false,
            wave: false,
            square_2: false,
            square_1: false,
            vin: false,
        }
    }

    pub fn set_vin_enable(&mut self, value: bool) {
        self.vin = value;
    }

    pub fn get_vin_enable(&self) -> bool {
        self.vin
    }

    pub fn write(&mut self, byte: u8) {
        self.noise = (byte & 0b1000) >> 3 != 0;
        self.wave = (byte & 0b100) >> 2 != 0;
        self.square_2 = (byte & 0b10) >> 1 != 0;
        self.square_1 = byte & 0b1 != 0;
    }

    pub fn read(&self) -> u8 {
        (if self.noise { 0b1000 } else { 0 })
            | (if self.wave { 0b100 } else { 0 })
            | (if self.square_2 { 0b10 } else { 0 })
            | (if self.square_1 { 1 } else { 0 })
    }

    pub fn mix(&self, voltages: ChannelsOutput) -> Voltage {
        let mut sum = Voltage(0);

        if self.square_1 {
            sum += voltages.square_1
        }
        if self.square_2 {
            sum += voltages.square_2
        }
        if self.wave {
            sum += voltages.wave
        }
        if self.noise {
            sum += voltages.noise
        }

        sum
    }
}

impl Default for Mixer {
    fn default() -> Self {
        Mixer::new()
    }
}

pub struct OutputBuffer {
    // output buffer
    buffer_index: usize,
    audio_available: bool,
    buffer: [AudioOutType; AUDIO_BUFFER_SIZE],
    buffer_2: [AudioOutType; AUDIO_BUFFER_SIZE],
}

impl OutputBuffer {
    pub fn new() -> Self {
        OutputBuffer {
            buffer_index: 0,
            audio_available: false,
            buffer: [0; AUDIO_BUFFER_SIZE],
            buffer_2: [0; AUDIO_BUFFER_SIZE],
        }
    }

    pub fn push(&mut self, voltage: Voltage) {
        self.buffer[self.buffer_index] = voltage.to_out_type();
        self.buffer_index += 1;

        if self.buffer_index == self.buffer.len() {
            // todo: actually, a callback should be called here
            self.audio_available = true;

            for i in 0..AUDIO_BUFFER_SIZE {
                self.buffer_2[i] = self.buffer[i] * VOLUME_BOOST as i16;
            }

            self.buffer_index = 0;
        }
    }

    // return the audio_buffer if it is filled
    pub fn get_audio_buffer(&mut self) -> Option<&[AudioOutType; AUDIO_BUFFER_SIZE]> {
        if !self.audio_available {
            return None;
        }
        self.audio_available = false;
        Some(&self.buffer_2)
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        OutputBuffer::new()
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
            sample_timer: Timer::new(CPU_FREQ / SAMPLE_RATE),

            left_sound_output: SoundOutput::new(),
            right_sound_output: SoundOutput::new(),

            power: false,
        }
    }

    pub fn tick(&mut self, t: u8) {
        for _i in 0..t {
            self.tick_channels();
            self.tick_frame_sequencer();
            self.tick_sample_timer();
        }
    }

    fn tick_channels(&mut self) {
        self.square_1.tick();
        self.square_2.tick();
        self.wave.tick();
        self.noise.tick();
    }

    fn tick_frame_sequencer(&mut self) {
        // if sequence timer has not finished/reached zero yet, return
        if !self.frame_sequencer.tick() {
            return;
        }

        // every 2 steps we tick the channel length counters
        if self.frame_sequencer.step % 2 == 0 {
            self.square_1.tick_length();
            self.square_2.tick_length();
            self.wave.tick_length();
            self.noise.tick_length();
        } else {
            self.square_1.half_tick_length();
            self.square_2.half_tick_length();
            self.wave.half_tick_length();
            self.noise.half_tick_length();
        }

        // at step 7, tick the channel envelopes
        if self.frame_sequencer.step == 7 {
            self.square_1.tick_envelope();
            self.square_2.tick_envelope();
            self.noise.tick_envelope();
        }

        // at step 2 and 6 tick the sweep
        if self.frame_sequencer.step == 2 || self.frame_sequencer.step == 6 {
            self.square_1.tick_sweep();
        }
    }

    fn tick_sample_timer(&mut self) {
        // sample timer not done yet? return
        if !self.sample_timer.tick() {
            return;
        }

        let mut channel_outputs = ChannelsOutput::new();

        if self.power {
            channel_outputs = ChannelsOutput {
                square_1: self.square_1.output(),
                square_2: self.square_2.output(),
                wave: self.wave.output(),
                noise: self.noise.output(),
            };
        }

        self.left_sound_output.receive(channel_outputs);
        // todo: what about right sound output?
    }

    pub fn get_audio_buffer(&mut self) -> Option<&[AudioOutType; AUDIO_BUFFER_SIZE]> {
        self.left_sound_output.out_buffer.get_audio_buffer()
    }

    // Square channel 1 sweep
    // NR10 FF10 -PPP NSSS Sweep period, negate, shift
    pub fn set_nr10(&mut self, value: u8) {
        if !self.power {
            return;
        }
        self.square_1.write_sweep(value);
    }

    pub fn get_nr10(&self) -> u8 {
        self.square_1.read_sweep()
    }

    // Square channel 1 duty and length load
    // NR11 FF11 DDLL LLLL Duty, Length load (64-L)
    pub fn set_nr11(&mut self, value: u8) {
        // on the DMG length counters are unaffected by power
        // and can still be written while power off
        if self.power {
            self.square_1.write_register_1(value);
        }
        self.square_1.length.set_value(value & 0b0011_1111);
    }

    pub fn get_nr11(&self) -> u8 {
        self.square_1.read_register_1()
    }

    // Square channel 1 envelope
    // NR12 FF12 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr12(&mut self, value: u8) {
        if !self.power {
            return;
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
            return;
        }
        self.square_1.set_frequency_lsb(value);
    }

    pub fn get_nr13(&self) -> u8 {
        0xFF
    }

    // Square channel 1 trigger, frequency MSB and length
    pub fn set_nr14(&mut self, value: u8) {
        if !self.power {
            return;
        }
        self.square_1.write_register_4(value);
    }

    pub fn get_nr14(&self) -> u8 {
        self.square_1.read_register_4()
    }

    // Square channel 2 duty and length load
    // NR21 FF16 DDLL LLLL Duty, Length load (64-L)
    pub fn set_nr21(&mut self, value: u8) {
        // on the DMG length counters are unaffected by power
        // and can still be written while power off
        if self.power {
            self.square_2.write_register_1(value);
        }
        self.square_2.length.set_value(value & 0b0011_1111);
    }

    pub fn get_nr21(&self) -> u8 {
        self.square_2.read_register_1()
    }

    // Square channel 2 envelope
    // NR22 FF17 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr22(&mut self, value: u8) {
        if !self.power {
            return;
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
            return;
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
            return;
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
            return;
        }

        self.wave.write_register_0(value);
    }

    pub fn get_nr30(&self) -> u8 {
        self.wave.read_register_0()
    }

    // Wave channel length load
    // NR31 FF1B LLLL LLLL Length load (256-L)
    pub fn set_nr31(&mut self, value: u8) {
        // on the DMG length counters are unaffected by power
        // and can still be written while power off
        self.wave.write_length_value(value)
    }

    pub fn get_nr31(&self) -> u8 {
        0xFF
    }

    // Wave channel volume
    // NR32 FF1C -VV- ---- Volume code (00=0%, 01=100%, 10=50%, 11=25%)
    pub fn set_nr32(&mut self, value: u8) {
        if !self.power {
            return;
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
            return;
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
            return;
        }
        self.wave.write_register_4(value);
    }

    pub fn get_nr34(&self) -> u8 {
        self.wave.read_register_4()
    }

    // Noise channel length load
    // NR41 FF20 --LL LLLL Length load (64-L)
    pub fn set_nr41(&mut self, value: u8) {
        // Oddity: While powered off, writes to NR41 are NOT ignored
        self.noise.write_length_value(value);
    }

    pub fn get_nr41(&self) -> u8 {
        0xFF
    }

    // Noise channel envelope
    // NR42 FF21 VVVV APPP Starting volume, Envelope add mode, period
    pub fn set_nr42(&mut self, value: u8) {
        if !self.power {
            return;
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
            return;
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
            return;
        }

        self.noise.write_register_4(value);
    }

    pub fn get_nr44(&self) -> u8 {
        self.noise.read_register_4()
    }

    // NR50 FF24 ALLL BRRR	Vin L enable, Left vol, Vin R enable, Right vol
    pub fn set_nr50(&mut self, byte: u8) {
        if !self.power {
            return;
        }

        self.left_sound_output
            .mixer
            .set_vin_enable((byte & 0b1000_0000) >> 7 != 0);
        self.right_sound_output
            .mixer
            .set_vin_enable((byte & 0b1000) >> 3 != 0);
        self.left_sound_output
            .volume_master
            .set_volume((byte & 0b0111_0000) >> 4);
        self.right_sound_output
            .volume_master
            .set_volume(byte & 0b111);
    }

    pub fn get_nr50(&self) -> u8 {
        (if self.left_sound_output.mixer.get_vin_enable() {
            0b1000_0000
        } else {
            0
        }) | (self.left_sound_output.volume_master.get_volume() << 4)
            | (if self.right_sound_output.mixer.get_vin_enable() {
                0b1000
            } else {
                0
            })
            | (self.right_sound_output.volume_master.get_volume())
    }

    // NR51 FF25 NW21 NW21 Left enables, Right enables
    pub fn set_nr51(&mut self, byte: u8) {
        if !self.power {
            return;
        }

        self.left_sound_output.mixer.write((byte & 0xF0) >> 4);
        self.right_sound_output.mixer.write(byte & 0xF);
    }

    pub fn get_nr51(&self) -> u8 {
        self.left_sound_output.mixer.read() << 4 | self.right_sound_output.mixer.read()
    }

    // NR52 FF26 P--- NW21 Power control/status, Channel length statuses
    pub fn set_nr52(&mut self, byte: u8) {
        let new_power = byte & 0b1000_0000 != 0;

        // power didn't change
        if new_power == self.power {
            return;
        }

        if new_power {
            // When powered on, the frame sequencer is reset so that the
            // next step will be 0, the square duty units are reset to the first step
            // of the waveform, and the wave channel's sample buffer is reset to 0.
            self.frame_sequencer.step = 7;
            self.square_1.duty_index = 0;
            self.square_2.duty_index = 0;
            self.wave.buffer = 0;
        } else {
            // When powered off, all
            // registers (NR10-NR51) are instantly written with zero and any writes to
            // those registers are ignored while power remains off (except on the DMG,
            // where length counters are unaffected by power and can still be written
            // while off)
            self.reset();
        }

        self.power = new_power;
    }

    pub fn get_nr52(&self) -> u8 {
        0b0111_0000
            | (if self.power { 0b1000_0000 } else { 0 })
            | (if self.noise.is_running() { 0b_1000 } else { 0 })
            | (if self.wave.is_running() { 0b_0100 } else { 0 })
            | (if self.square_2.is_running() { 0b_10 } else { 0 })
            | (if self.square_1.is_running() { 1 } else { 0 })
    }

    // called when power is set to off, through register nr52
    pub fn reset(&mut self) {
        self.left_sound_output = SoundOutput::new();
        self.right_sound_output = SoundOutput::new();

        self.set_nr10(0);
        self.set_nr11(0);
        self.set_nr12(0);
        self.set_nr13(0);
        self.set_nr14(0);

        self.set_nr21(0);
        self.set_nr22(0);
        self.set_nr23(0);
        self.set_nr24(0);

        self.set_nr30(0);
        self.set_nr31(0);
        self.set_nr32(0);
        self.set_nr33(0);
        self.set_nr34(0);

        // powering off shouldn't affect NR41
        self.set_nr42(0);
        self.set_nr43(0);
        self.set_nr44(0);

        self.set_nr50(0);
        self.set_nr51(0);

        // reset channels
        self.square_1.reset();
        self.square_2.reset();

        self.wave.reset();
        self.noise.reset();
    }
}

impl Default for Sound {
    fn default() -> Self {
        Sound::new()
    }
}

pub struct FrameSequencer {
    timer: Timer,
    step: u8, // goes up by 1 everytime the timer hits 0
}

impl FrameSequencer {
    pub fn new() -> Self {
        FrameSequencer {
            // it runs at 512hz, CPU runs at 4194304hz, 4194304/512=8192
            timer: Timer::new(8192),
            step: 0,
        }
    }

    // ticks the timer and increases step when the timer hits 0
    pub fn tick(&mut self) -> bool {
        let timer_up = self.timer.tick();
        if timer_up {
            self.step = (self.step + 1) % DUTY_PATTERNS_LENGTH;
        }
        timer_up
    }

    pub fn reset(&mut self) {
        self.step = 0;
        self.timer.restart();
    }
}

impl Default for FrameSequencer {
    fn default() -> Self {
        FrameSequencer::new()
    }
}

#[derive(Clone, Copy)]
// a timer with a default period of 8
pub struct TimerDefaultPeriod {
    period: usize, // initial and max value of curr
    curr: usize,   // goes down by 1 every tick and wraps back to period
}

impl TimerDefaultPeriod {
    pub fn new() -> Self {
        TimerDefaultPeriod { period: 0, curr: 0 }
    }

    pub fn tick(&mut self) -> bool {
        if self.curr == 0 {
            return false;
        }

        self.curr -= 1;

        if self.curr == 0 {
            self.restart();
            return true;
        }

        false
    }

    pub fn get_period(&self) -> usize {
        if self.period != 0 {
            self.period
        } else {
            8
        }
    }

    pub fn set_period(&mut self, period: usize) {
        self.period = period;
    }

    pub fn restart(&mut self) {
        self.curr = self.get_period()
    }
}

impl Default for TimerDefaultPeriod {
    fn default() -> Self {
        TimerDefaultPeriod::new()
    }
}

#[derive(Clone, Copy)]
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
        // the timer is not initialized yet
        if self.period == 0 {
            return false;
        }

        self.curr = self.curr.wrapping_sub(1);

        if self.curr == 0 {
            self.restart();
            return true;
        }

        false
    }

    pub fn restart(&mut self) {
        self.curr = self.period;
    }
}
