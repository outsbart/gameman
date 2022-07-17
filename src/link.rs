/// Link cable

pub struct Link {
    buffer_out: [char; 256],
    buffer_index: usize,
    data: u8,
    control: u8,
}

impl Link {
    pub fn new() -> Self {
        Link {
            buffer_out: [char::from(32); 256],
            buffer_index: 0,
            data: 0,
            control: 0,
        }
    }

    pub fn set_data(&mut self, byte: u8) {
        self.data = byte;
    }

    pub fn set_control(&mut self, byte: u8) {
        self.control = byte;
        if byte == 0x81 {
            self.send();
        }
    }

    pub fn get_data(&self) -> u8 {
        self.data
    }

    pub fn get_control(&self) -> u8 {
        self.control
    }

    fn send(&mut self) {
        self.buffer_out[self.buffer_index] = self.data as char;
        self.buffer_index = (self.buffer_index + 1) % 256;
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

    #[test]
    fn send() {
        let mut link = Link::new();

        link.set_data(b'w');
        link.send();
        link.set_data(b'o');
        link.send();
        link.set_data(b'w');
        link.send();

        assert_eq!(link.get_buffer()[0], 'w');
        assert_eq!(link.get_buffer()[1], 'o');
        assert_eq!(link.get_buffer()[2], 'w');
    }
}
