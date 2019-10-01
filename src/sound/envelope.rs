use sound::{Sample, TimerDefaultPeriod};
use std::ops::{Add, Sub};

const VOLUME_MAX: Sample = Sample(0xF);
const VOLUME_MIN: Sample = Sample(0);


// every tick, increases or decreases volume
#[derive(Clone, Copy)]
pub struct Envelope {
    timer: TimerDefaultPeriod,
    pub add_mode: bool,
    volume: Sample,
    pub volume_initial: Sample,
}


impl Envelope {
    pub fn new() -> Self {
        Envelope {
            timer: TimerDefaultPeriod::new(),
            add_mode: false,
            volume: Sample(0),
            volume_initial: Sample(0),
        }
    }

    pub fn get_volume(&self) -> Sample {
        self.volume
    }

    pub fn trigger(&mut self) {
        // Volume envelope timer is reloaded with period
        self.timer.restart();

        // Channel volume is reloaded from NRx2
        self.volume = self.volume_initial;
    }

    pub fn write(&mut self, byte: u8) {
        self.timer.period = (byte & 0b111) as usize;

        self.add_mode = byte & 0b1000 != 0;
        self.volume_initial = Sample(byte >> 4);
    }

    pub fn read(&self) -> u8 {
        self.timer.period as u8 | (if self.add_mode == true { 0b1000 } else { 0 }) | (u8::from(self.volume_initial) << 4)
    }

    pub fn tick(&mut self) {
        if self.timer.period == 0 {
            return;
        }

        // when timer runs out
        if self.timer.tick() {

            // must disable on overflow/underflow
            if (self.add_mode && self.volume == VOLUME_MAX) || (!self.add_mode && self.volume == VOLUME_MIN) {
                return;
            }

            // increase or decrease based on add_mode
            let operation = if self.add_mode { Sample::add } else { Sample::sub };

            self.volume = operation(self.volume, Sample(1));
        }
    }
}
