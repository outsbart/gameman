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
struct Timer {
    value: u8,
}

#[derive(Default)]
struct FallingEdgeDetector {
    prev_signal: bool,
    curr_signal: bool,
}

impl FallingEdgeDetector {
    fn bit_for_speed(speed: TimerSpeed) -> u16 {
        match speed {
            TimerSpeed::Speed0 => 0b1000000000,
            TimerSpeed::Speed1 => 0b1000,
            TimerSpeed::Speed2 => 0b100000,
            TimerSpeed::Speed3 => 0b10000000,
        }
    }

    fn compute_and_detect(&mut self, speed: TimerSpeed, running: bool, divider: u16) -> bool {
        let bit_to_check = Self::bit_for_speed(speed);

        self.prev_signal = self.curr_signal;
        self.curr_signal = running && (divider & bit_to_check != 0);

        // returns true if the value changed from true to false
        (self.prev_signal) && !self.curr_signal
    }

    // Immediate edge check when TAC is written — hardware updates the
    // multiplexer output synchronously with the write, before the next tick.
    // Returns true if a falling edge occurred (caller handles TIMA increment).
    fn check_write_edge(&mut self, speed: TimerSpeed, running: bool, divider: u16) -> bool {
        let bit_to_check = Self::bit_for_speed(speed);
        let new_signal = running && (divider & bit_to_check != 0);
        let falling_edge = self.curr_signal && !new_signal;
        self.curr_signal = new_signal;
        falling_edge
    }
}

impl Timer {
    fn increase(&mut self) -> bool {
        // returns true if overflowed
        self.value = self.value.wrapping_add(1);
        self.value == 0
    }
}

pub struct Timers {
    // tac
    speed: TimerSpeed,
    running: bool, // true if enabled

    // registers
    divider: u16,
    tma: u8, // modulo
    tima: Timer,

    tima_reload_cycle: u8, // keep track of the status of reloading after tima overflows
    falling_edge_detector: FallingEdgeDetector,
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            divider: 0xABCC,
            tma: 0,
            speed: TimerSpeed::Speed0,
            running: false,

            tima: Timer::default(),
            tima_reload_cycle: 0,
            falling_edge_detector: FallingEdgeDetector::default(),
        }
    }

    fn tima_reloader_tick(&mut self) -> bool {
        // returns true if tima got reloaded
        if self.tima_reload_cycle == 0 {
            return false;
        }

        self.tima_reload_cycle = self.tima_reload_cycle.wrapping_sub(1);

        if self.tima_reload_cycle == 4 {
            self.tima.value = self.tma;
            return true;
        }

        false
    }

    fn get_tima_reload_status(&self) -> TimaStatus {
        if self.tima_reload_cycle > 0 && self.tima_reload_cycle <= 4 {
            return TimaStatus::BeingReloaded;
        }
        if self.tima_reload_cycle > 4 && self.tima_reload_cycle <= 8 {
            return TimaStatus::JustReloaded;
        }
        TimaStatus::NotBusy
    }

    // returns true if timer interrupt should be triggered
    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut interrupt = false;

        for _ in 0..cycles {
            self.divider = self.divider.wrapping_add(1);

            interrupt |= self.tima_reloader_tick();

            if self.get_tima_reload_status() != TimaStatus::NotBusy {
                continue;
            }

            if self
                .falling_edge_detector
                .compute_and_detect(self.speed, self.running, self.divider)
            {
                // tima overflowed
                if self.tima.increase() {
                    self.tima_reload_cycle = 8;
                }
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
        match self.get_tima_reload_status() {
            TimaStatus::BeingReloaded => {
                // ignore the value when reloading
                return;
            }
            TimaStatus::JustReloaded => {
                self.tima_reload_cycle = 0;
            }
            TimaStatus::NotBusy => {}
        }

        self.tima.value = byte;
    }

    // when writing to 0xFF06
    pub fn write_tma(&mut self, byte: u8) {
        // load tima too if already reloading tima
        if self.get_tima_reload_status() == TimaStatus::BeingReloaded {
            self.tima.value = byte;
        }

        self.tma = byte;
    }

    // when writing to 0xFF07
    pub fn write_tac(&mut self, byte: u8) {
        self.speed = TimerSpeed::from_u8(byte & 0b0000_0011);
        self.running = ((byte & 0b0000_0100) >> 2) == 1;
        // Hardware updates the multiplexer output on the same cycle as the write.
        // If it falls from 1→0, TIMA increments (and the reload/interrupt pipeline starts).
        if self.falling_edge_detector.check_write_edge(self.speed, self.running, self.divider)
            && self.tima.increase() {
                self.tima_reload_cycle = 8;
            }
    }

    pub fn divider(&self) -> u16 {
        self.divider
    }

    // when reading from 0xFF04
    pub fn read_divider(&self) -> u8 {
        (self.divider >> 8) as u8
    }

    // when writing to 0xFF05
    pub fn read_tima(&self) -> u8 {
        self.tima.value
    }

    // when reading from 0xFF06
    pub fn read_tma(&self) -> u8 {
        self.tma
    }

    // when reading from 0xFF07
    pub fn read_tac(&self) -> u8 {
        0xF8 | (if self.running { 0b100 } else { 0 }) | (self.speed as u8)
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

        assert_eq!(timers.divider, 0xABCC);
        assert_eq!(timers.tima.value, 0);
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

        assert_eq!(timers.read_tac(), 0b1111_1111);
    }
}
