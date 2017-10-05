struct Clocks {
    m: i64, t: i64
}

impl Clocks {
    fn new() -> Clocks {
        Clocks { m: 0, t: 0 }
    }
}

struct Registers {
    a: u8, b: u8, c: u8, d: u8,
    e: u8, h: u8, l: u8, f: u8,

    pc: u16, sp: u16,
    m: i64, t: i64
}

impl Registers {
    fn new() -> Registers {
        Registers {
            a: 0, b: 0, c: 0, d: 0,
            e: 0, h: 0, l:0, f: 0,

            pc: 0, sp: 0,
            m: 0, t: 0
        }
    }
}

pub struct CPU {
    clocks: Clocks,
    registers: Registers
}

impl CPU {
    pub fn new() -> CPU {
        CPU { clocks: Clocks::new(), registers: Registers::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_inizialization() {
        let CPU { clocks, registers } = CPU::new();

        assert_eq!(clocks.m, 0);
        assert_eq!(clocks.t, 0);

        assert_eq!(registers.a, 0);
        assert_eq!(registers.b, 0);
        assert_eq!(registers.c, 0);
        assert_eq!(registers.d, 0);
        assert_eq!(registers.e, 0);
        assert_eq!(registers.h, 0);
        assert_eq!(registers.l, 0);
        assert_eq!(registers.f, 0);
        assert_eq!(registers.pc, 0);
        assert_eq!(registers.sp, 0);
        assert_eq!(registers.m, 0);
        assert_eq!(registers.t, 0);
    }
}