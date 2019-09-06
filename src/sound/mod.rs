use mem::Memory;

pub struct Sound {
    channel_1: SquareChannel,
    channel_2: SquareChannel,
    channel_3: WaveChannel,
    channel_4: NoiseChannel
}


impl Memory for Sound {
    fn read_byte(&mut self, addr: u16) -> u8 {
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

}

impl Envelope {
    pub fn new() -> Self {
        Envelope {}
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
}

impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new()
        }
    }
}

struct WaveChannel {

}

impl WaveChannel {
    pub fn new() -> Self {
        WaveChannel { }
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

}
