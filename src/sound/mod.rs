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

    pub duty: u8,
    pub length_load: u8,
}

impl SquareChannel {
    pub fn new() -> Self {
        SquareChannel {
            sweep: Sweep::new(),
            envelope: Envelope::new(),
            duty: 0,
            length_load: 0
        }
    }

    pub fn write_register_1(&mut self, byte: u8) {
        self.length_load = byte & 0b0011_1111;
        self.duty = (byte & 0b1100_0000) >> 6;
    }

    pub fn read_register_1(&self) -> u8 {
        (self.duty << 6) | self.length_load
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

}
