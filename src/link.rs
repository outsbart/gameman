/// Link cable

pub struct Link {
    buffer_out: [char; 256],
    buffer_index: usize,
}

impl Link {
    pub fn new() -> Self {
        Link {
            buffer_out: [char::from(32); 256],
            buffer_index: 0,
        }
    }

    pub fn send(&mut self, value: char) {
        warn!("New char on the serial port: {}", value);
        self.buffer_out[self.buffer_index] = value;
        self.buffer_index = (self.buffer_index + 1) % 256;
    }

    pub fn get_buffer(&self) -> [char; 256] {
        self.buffer_out
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

        link.send('w');
        link.send('o');
        link.send('w');

        assert_eq!(link.get_buffer()[0], 'w');
        assert_eq!(link.get_buffer()[1], 'o');
        assert_eq!(link.get_buffer()[2], 'w');
    }
}
