use crate::sound::{Sample, TimerDefaultPeriod};
use crate::utils::{bit, bit_if, field, hi_nibble};
use serde::{Deserialize, Serialize};

// every tick, increases or decreases volume
#[derive(Clone, Copy, Serialize, Deserialize)]
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
        self.timer.period = field(byte, 0, 3) as usize;
        self.add_mode = bit(byte, 3);
        self.volume_initial = Sample(hi_nibble(byte));
    }

    pub fn read(&self) -> u8 {
        self.timer.period as u8 | bit_if(self.add_mode, 3) | (u8::from(self.volume_initial) << 4)
    }

    pub fn tick(&mut self) {
        // not initialized
        if self.timer.period == 0 {
            return;
        }

        // timer still not zero
        if !self.timer.tick() {
            return;
        }

        // increase or decrease based on add_mode
        // value stays between SAMPLE_MIN and SAMPLE_MAX
        if self.add_mode {
            self.volume.increase();
        } else {
            self.volume.decrease();
        };
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope::new()
    }
}
