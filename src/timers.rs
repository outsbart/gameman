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

pub struct Timers {
    // tac
    speed: TimerSpeed,
    running: bool, // true if enabled

    // registers
    divider: u16,
    tima: u8, // counter
    tma: u8,  // modulo

    prev_signal: bool, // falling edge detector
    cycles_until_reloading_tima: u8,
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            divider: 0,
            tima: 0,
            tma: 0,
            speed: TimerSpeed::Speed0,
            running: false,

            prev_signal: false,
            cycles_until_reloading_tima: 0,
        }
    }

    // send the timers forward; returns true if timer interrupt should be triggered
    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut interrupt = false;

        let bit_to_check: u16 = match self.speed {
            TimerSpeed::Speed0 => 0b1000000000,
            TimerSpeed::Speed1 => 0b1000,
            TimerSpeed::Speed2 => 0b100000,
            TimerSpeed::Speed3 => 0b10000000,
        };

        for _ in 0..cycles {
            self.divider = self.divider.wrapping_add(1);

            if self.cycles_until_reloading_tima > 0 {
                self.cycles_until_reloading_tima = self.cycles_until_reloading_tima.wrapping_sub(1);
            }

            interrupt |= self.reload_tima_if_necessary();

            // no need to check edge detector
            if self.cycles_until_reloading_tima > 0 {
                continue;
            }

            let signal = self.running && (self.divider & bit_to_check != 0);

            // falling edge detector
            if (self.prev_signal == true) && !signal {
                self.tima = self.tima.wrapping_add(1);

                // tima overflowed
                if self.tima == 0 {
                    // for 4 cycles tima is 0, then it is reloaded from tma
                    self.cycles_until_reloading_tima = 8;
                }
            }

            self.prev_signal = signal;
        }

        interrupt
    }

    fn reload_tima_if_necessary(&mut self) -> bool {
        // returns true if interrupt should be triggered
        // tima should be reloaded after 4 cycles
        if self.cycles_until_reloading_tima == 4 {
            self.tima = self.tma;
            return true;
        }

        false
    }

    // when writing to 0xFF04
    pub fn change_divider(&mut self, _byte: u8) {
        // always resets
        self.divider = 0;
    }

    // when writing to 0xFF05
    pub fn write_tima(&mut self, byte: u8) {
        if self.cycles_until_reloading_tima > 0 && self.cycles_until_reloading_tima <= 4 {
            return;
        }

        // if self.cycles_until_reloading_tima > 4 && self.cycles_until_reloading_tima <= 8 {
        //     self.cycles_until_reloading_tima = 0;
        //     return;
        // }

        self.tima = byte;
    }

    // when writing to 0xFF06
    pub fn write_tma(&mut self, byte: u8) {
        if self.cycles_until_reloading_tima > 0 && self.cycles_until_reloading_tima <= 4 {
            self.tima = byte;
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
        self.tima
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
        assert_eq!(timers.tima, 0);
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
