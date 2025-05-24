/// Link cable

// Post-boot divider = 0xABCC; bit 8 starts high.
const POST_BOOT_BIT8: bool = (0xABCCu16 & 0x100) != 0;

pub struct Link {
    buffer_out: [char; 256],
    buffer_index: usize,
    data: u8,
    control: u8,
    transferring: bool,
    shift_count: u8,
    prev_bit8: bool,
}

impl Link {
    pub fn new() -> Self {
        Link {
            buffer_out: [char::from(32); 256],
            buffer_index: 0,
            data: 0,
            control: 0x7E,
            transferring: false,
            shift_count: 0,
            prev_bit8: POST_BOOT_BIT8,
        }
    }

    pub fn set_data(&mut self, byte: u8) {
        self.data = byte;
    }

    pub fn set_control(&mut self, byte: u8) {
        self.control = byte;
        // SC = 0x81: transfer requested with internal clock
        if byte & 0x81 == 0x81 {
            self.transferring = true;
            self.shift_count = 0;
        }
    }

    pub fn get_data(&self) -> u8 {
        self.data
    }

    pub fn get_control(&self) -> u8 {
        self.control
    }

    // Advance the serial clock by one T-cycle. Returns true when a byte transfer
    // completes and the serial interrupt (IF bit 3) should be requested.
    // The serial clock is driven by the falling edge of bit 8 of the internal divider
    // (8192 Hz = one edge every 512 T-cycles). Call this AFTER timers.tick().
    pub fn tick(&mut self, divider: u16) -> bool {
        let bit8 = (divider & 0x100) != 0;
        let falling_edge = self.prev_bit8 && !bit8;
        self.prev_bit8 = bit8;

        if !self.transferring || !falling_edge {
            return false;
        }

        self.shift_count += 1;
        if self.shift_count >= 8 {
            self.shift_count = 0;
            self.transferring = false;
            self.control &= 0x7F; // clear transfer-start bit
            self.buffer_out[self.buffer_index] = self.data as char;
            self.buffer_index = (self.buffer_index + 1) % 256;
            return true;
        }

        false
    }

    pub fn get_buffer(&self) -> [char; 256] {
        self.buffer_out
    }
}

impl Default for Link {
    fn default() -> Self {
        Link::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_inizialization() {
        let link = Link::new();

        assert_eq!(link.buffer_out[0], ' ');
        assert_eq!(link.buffer_out[255], ' ');
        assert_eq!(link.buffer_index, 0);
    }
}
