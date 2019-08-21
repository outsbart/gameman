#[derive(Clone, Copy)]
#[repr(u8)]
enum TimerSpeed {
    Speed0 = 0,
    Speed1 = 1,
    Speed2 = 2,
    Speed3 = 3
}

impl TimerSpeed {
    pub fn from_u8(byte: u8) -> TimerSpeed {
        match byte {
            0b00 => TimerSpeed::Speed0,
            0b01 => TimerSpeed::Speed1,
            0b10 => TimerSpeed::Speed2,
            0b11 => TimerSpeed::Speed3,
            _ => { panic!("Unable to set timer speed"); }
        }
    }
}

impl Into<u8> for TimerSpeed {
    fn into(self) -> u8 {
        match self {
            TimerSpeed::Speed0 => 0,
            TimerSpeed::Speed1 => 1,
            TimerSpeed::Speed2 => 0b10,
            TimerSpeed::Speed3 => 0b11
        }
    }
}


pub struct Timers {
    main: u8,
    sub: u8,

    // value of divider register
    divider: u8,
    counter: u8,
    modulo: u8,
    speed: TimerSpeed,
    // true if enabled
    running: bool
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            main: 0,
            sub: 0,
            divider: 0,
            counter: 0,
            modulo: 0,
            speed: TimerSpeed::Speed0,
            running: false
        }
    }

    // send the timers forward; returns true if timer interrupt should be triggered
    pub fn tick(&mut self, cycles: u8) {
    }

    // when writing to 0xFF04
    pub fn change_divider(&mut self, _byte: u8) {
        // always resets
        self.divider = 0;
    }

    // when writing to 0xFF05
    pub fn change_counter(&mut self, byte: u8) {
        self.counter = byte;
    }

    // when writing to 0xFF06
    pub fn change_modulo(&mut self, byte: u8) {
        self.modulo = byte;
    }

    // when writing to 0xFF07
    pub fn change_control(&mut self, byte: u8) {
        self.speed = TimerSpeed::from_u8(byte & 0b0000_0011);
        self.running = ((byte & 0b0000_0100) >> 2) == 1;
    }

    // when reading from 0xFF04
    pub fn read_divider(&self) -> u8 {
        self.divider
    }

    // when writing to 0xFF05
    pub fn read_counter(&self) -> u8 {
        self.counter
    }

    // when reading from 0xFF06
    pub fn read_modulo(&self) -> u8 {
        self.modulo
    }

    // when reading from 0xFF07
    pub fn read_control(&self) -> u8 {
        (if self.running { 0b100 } else { 0 } ) | (self.speed as u8)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timers_initialization() {
        let timers = Timers::new();

        assert_eq!(timers.divider, 0);
        assert_eq!(timers.counter, 0);
        assert_eq!(timers.modulo, 0);
        assert_eq!(timers.speed as u8, 0);
        assert_eq!(timers.running, false);
    }

    #[test]
    fn test_divider_access() {
        let mut timers = Timers::new();

        // should set it to 0
        timers.change_divider(4);

        assert_eq!(timers.read_divider(), 0)
    }

    #[test]
    fn test_counter_access() {
        let mut timers = Timers::new();

        timers.change_counter(4);

        assert_eq!(timers.read_counter(), 4)
    }

    #[test]
    fn test_modulo_access() {
        let mut timers = Timers::new();

        timers.change_modulo(5);

        assert_eq!(timers.read_modulo(), 5)
    }

    #[test]
    fn test_timer_control_access() {
        let mut timers = Timers::new();

        timers.change_control(0b0000_0111);

        assert_eq!(timers.running, true);
        assert_eq!(timers.speed as u8, 0b11);

        assert_eq!(timers.read_control(), 0b0000_0111);
    }
}
