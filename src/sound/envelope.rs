use sound::{Sample};

const VOLUME_MAX: Sample = 0xF;
const VOLUME_MIN: Sample = 0;


#[derive(Clone,Copy)]
pub struct EnvelopeTimer {
    pub period: usize, // initial and max value of curr
    curr: usize,       // goes down by 1 every tick and wraps back to period
}

impl EnvelopeTimer {
    pub fn new() -> Self {
        EnvelopeTimer {
            period: 0,
            curr: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.curr -= 1;

        if self.curr == 0 {
            self.restart();
            return true;
        }

        return false;
    }

    pub fn restart(&mut self) {
        self.curr = if self.period != 0 { self.period } else { 8 }
    }
}


// every tick, increases or decreases volume
#[derive(Clone,Copy)]
pub struct Envelope {
    timer: EnvelopeTimer,
    pub add_mode: bool,
    volume: u8,
    pub volume_initial: u8,
    enabled: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            timer: EnvelopeTimer::new(),
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
        self.timer.restart();

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
            self.volume = if self.add_mode {
                self.volume + 1
            } else {
                self.volume - 1
            };
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope() {
        let mut envelope: Envelope = Envelope::new();

        assert_eq!(envelope.read(), 0);

        envelope.write(0b1000_1011);
        assert_eq!(envelope.period, 0b011);
        assert_eq!(envelope.add_mode, true);
        assert_eq!(envelope.volume, 0b1000);

        envelope.volume = 0b1110;
        envelope.add_mode = false;
        envelope.period = 0b111;

        assert_eq!(envelope.read(), 0b1110_0111);
    }
}
