#[derive(Clone, Copy)]
#[repr(u8)]
enum TimerSpeed {
    Speed0 = 0,
    Speed1 = 1,
    Speed2 = 2,
    Speed3 = 3,
}

impl TimerSpeed {
    pub fn from_u8(byte: u8) -> TimerSpeed {
        match byte {
            0b00 => TimerSpeed::Speed0,
            0b01 => TimerSpeed::Speed1,
            0b10 => TimerSpeed::Speed2,
            0b11 => TimerSpeed::Speed3,
            _ => {
                panic!("Unable to set timer speed");
            }
        }
    }
}

impl From<u8> for TimerSpeed {
    fn from(val: u8) -> Self {
        match val {
            0 => TimerSpeed::Speed0,
            1 => TimerSpeed::Speed1,
            0b10 => TimerSpeed::Speed2,
            0b11 => TimerSpeed::Speed3,
            _ => panic!("Impossible timer speed"),
        }
    }
}

#[derive(PartialEq)]
enum TimaStatus {
    BeingReloaded,
    JustReloaded,
    NotBusy,
}

#[derive(Default)]
struct Tima {
    cycles: u8, // keep track of the status after overflowing
    _value: u8,
}

#[derive(Default)]
struct FallingEdgeDetector {
    prev_signal: bool,
    curr_signal: bool,
}

impl FallingEdgeDetector {
    fn triggered(&mut self) -> bool {
        // returns true if the value changed from true to false
        (self.prev_signal == true) && !self.curr_signal
    }

    fn compute_signal(&mut self, speed: TimerSpeed, running: bool, divider: u16) {
        let bit_to_check: u16 = match speed {
            TimerSpeed::Speed0 => 0b1000000000,
            TimerSpeed::Speed1 => 0b1000,
            TimerSpeed::Speed2 => 0b100000,
            TimerSpeed::Speed3 => 0b10000000,
        };

        self.prev_signal = self.curr_signal;
        self.curr_signal = running && (divider & bit_to_check != 0);
    }
}

impl Tima {
    fn get_status(&self) -> TimaStatus {
        if self.cycles > 0 && self.cycles <= 4 {
            return TimaStatus::BeingReloaded;
        }
        if self.cycles > 4 && self.cycles <= 8 {
            return TimaStatus::JustReloaded;
        }
        TimaStatus::NotBusy
    }

    fn tick(&mut self, tma: u8) -> bool {
        // returns true if tima got reloaded
        if self.cycles > 0 {
            self.cycles = self.cycles.wrapping_sub(1);
        }

        if self.cycles == 4 {
            self._value = tma;
            return true;
        }

        false
    }

    fn increase(&mut self) {
        self._value = self._value.wrapping_add(1);

        // overflow
        if self._value == 0 {
            // for 4 cycles tima is 0, then it is reloaded from tma
            self.cycles = 8;
        }
    }

    fn get_value(&self) -> u8 {
        self._value
    }

    fn set_value(&mut self, value: u8) {
        match self.get_status() {
            TimaStatus::BeingReloaded => {
                // ignore the value when reloading
                return;
            }
            TimaStatus::JustReloaded => {
                self.cycles = 0;
            }
            TimaStatus::NotBusy => {}
        }

        self._value = value;
    }
}

pub struct Timers {
    // tac
    speed: TimerSpeed,
    running: bool, // true if enabled

    // registers
    divider: u16,
    tma: u8, // modulo
    tima: Tima,

    falling_edge_detector: FallingEdgeDetector,
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            divider: 0,
            tma: 0,
            speed: TimerSpeed::Speed0,
            running: false,

            tima: Tima::default(),

            falling_edge_detector: FallingEdgeDetector::default(),
        }
    }

    // send the timers forward; returns true if timer interrupt should be triggered
    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut interrupt = false;

        for _ in 0..cycles {
            self.divider = self.divider.wrapping_add(1);

            interrupt |= self.tima.tick(self.tma);

            if self.tima.get_status() != TimaStatus::NotBusy {
                continue;
            }

            self.falling_edge_detector
                .compute_signal(self.speed, self.running, self.divider);

            if self.falling_edge_detector.triggered() {
                self.tima.increase(); // can overflow
            }
        }

        interrupt
    }

    // when writing to 0xFF04
    pub fn change_divider(&mut self, _byte: u8) {
        // always resets
        self.divider = 0;
    }

    // when writing to 0xFF05
    pub fn write_tima(&mut self, byte: u8) {
        self.tima.set_value(byte);
    }

    // when writing to 0xFF06
    pub fn write_tma(&mut self, byte: u8) {
        // load tima too if already reloading tima
        if self.tima.get_status() == TimaStatus::BeingReloaded {
            self.tima._value = byte;
        }

        self.tma = byte;
    }

    // when writing to 0xFF07
    pub fn write_tac(&mut self, byte: u8) {
        self.speed = TimerSpeed::from_u8(byte & 0b0000_0011);
        self.running = ((byte & 0b0000_0100) >> 2) == 1;
    }

    // when reading from 0xFF04
    pub fn read_divider(&self) -> u8 {
        (self.divider >> 8) as u8
    }

    // when writing to 0xFF05
    pub fn read_tima(&self) -> u8 {
        self.tima.get_value()
    }

    // when reading from 0xFF06
    pub fn read_tma(&self) -> u8 {
        self.tma
    }

    // when reading from 0xFF07
    pub fn read_tac(&self) -> u8 {
        (if self.running { 0b100 } else { 0 }) | (self.speed as u8)
    }
}

impl Default for Timers {
    fn default() -> Self {
        Timers::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timers_initialization() {
        let timers = Timers::new();

        assert_eq!(timers.divider, 0);
        assert_eq!(timers.tima.get_value(), 0);
        assert_eq!(timers.tma, 0);
        assert_eq!(timers.speed as u8, 0);
        assert!(!timers.running);
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

        timers.write_tima(4);

        assert_eq!(timers.read_tima(), 4)
    }

    #[test]
    fn test_modulo_access() {
        let mut timers = Timers::new();

        timers.write_tma(5);

        assert_eq!(timers.read_tma(), 5)
    }

    #[test]
    fn test_timer_control_access() {
        let mut timers = Timers::new();

        timers.write_tac(0b0000_0111);

        assert!(timers.running);
        assert_eq!(timers.speed as u8, 0b11);

        assert_eq!(timers.read_tac(), 0b0000_0111);
    }
}
