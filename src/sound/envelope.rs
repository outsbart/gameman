use sound::{Timer, Sample};

const VOLUME_MAX: Sample = 0xF;
const VOLUME_MIN: Sample = 0;


// every tick, increases or decreases volume
#[derive(Clone,Copy)]
pub struct Envelope {
    timer: Timer,
    add_mode: bool,
    period: u8,
    volume: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            timer: Timer::new(0),
            add_mode: false,
            period: 0,
            volume: 0,
        }
    }

    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    pub fn dac_enabled(&self) -> bool {
        self.add_mode != false || self.volume != 0
    }

    pub fn write(&mut self, byte: u8) {
        self.period = byte & 0b111;
        self.add_mode = byte & 0b1000 != 0;
        self.volume = byte >> 4;

        self.timer = Timer::new(self.period as usize);
    }

    pub fn read(&self) -> u8 {
        self.period | (if self.add_mode == true { 0b1000 } else { 0 }) | (self.volume << 4)
    }

    pub fn tick(&mut self) {
        if self.period == 0 {
            return
        }

        // when timer runs out
        if self.timer.tick() {
            if self.add_mode && self.volume < VOLUME_MAX {
                self.volume += 1;
            }
            else if !self.add_mode && self.volume > VOLUME_MIN {
                self.volume -= 1;
            }
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
