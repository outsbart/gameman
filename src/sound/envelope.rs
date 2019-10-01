use sound::{Sample, TimerDefaultPeriod};
use std::ops::{Add, Sub};

const VOLUME_MAX: Sample = 0xF;
const VOLUME_MIN: Sample = 0;



// every tick, increases or decreases volume
#[derive(Clone,Copy)]
pub struct Envelope {
    timer: TimerDefaultPeriod,
    pub add_mode: bool,
    volume: u8,
    pub volume_initial: u8,
    enabled: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            timer: TimerDefaultPeriod::new(),
            add_mode: false,
            volume: 0,
            volume_initial: 0,
            enabled: false,
        }
    }

    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    pub fn trigger(&mut self) {
        // Volume envelope timer is reloaded with period
        self.timer.restart();

        // Channel volume is reloaded from NRx2
        self.volume = self.volume_initial;

        self.enabled = true;
    }

    pub fn write(&mut self, byte: u8) {
        self.timer.period = (byte & 0b111) as usize;
        self.timer.restart();  // todo: make sure this is necessary

        self.add_mode = byte & 0b1000 != 0;
        self.volume_initial = byte >> 4;
    }

    pub fn read(&self) -> u8 {
        self.timer.period as u8 | (if self.add_mode == true { 0b1000 } else { 0 }) | (self.volume_initial << 4)
    }

    pub fn tick(&mut self) {
        if !self.enabled {
            return
        }

        // when timer runs out
        if self.timer.tick() {

            // must disable on overflow/underflow
            if (self.add_mode && self.volume == VOLUME_MAX) || (!self.add_mode && self.volume == VOLUME_MIN) {
                self.enabled = false;
                return;
            }

            // increase or decrease based on add_mode
            let operation = if self.add_mode { u8::add } else { u8::sub };

            self.volume = operation(self.volume, 1);
        }
    }
}
