use mem::Memory;

// Flags poisition in the F register
const ZERO_FLAG: u8 = 0x80;
const OPERATION_FLAG: u8 = 0x40;
const HALF_CARRY_FLAG: u8 = 0x20;
const CARRY_FLAG: u8 = 0x10;


struct Clocks {
    m: u32, t: u32  // TODO: check if i32 is the right type
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
    m: u8, t: u8
}

impl Registers {
    fn new() -> Registers {
        Registers {
            a: 0, b: 0, c: 0, d: 0,
            e: 0, h: 0, l:0, f: 0,

            pc: 0, sp: 20, // TODO: change sp value
            m: 0, t: 0
        }
    }
}

pub struct CPU<M: Memory> {
    clks: Clocks,
    regs: Registers,

    mmu: M
}

impl<M: Memory> CPU<M> {
    pub fn new(mmu: M) -> CPU<M> {
        CPU { clks: Clocks::new(), regs: Registers::new(), mmu }
    }

    // operations

    // adds E to A
    fn addr_e(&mut self) {
        let result = self.regs.a as u32 + self.regs.e as u32;

        self.regs.f = 0; // reset the flags!

        // Zero
        if (result & 0xFF) == 0 {
            self.regs.f |= ZERO_FLAG; // if result is 0 set the first bit to 1
        }

        // Half Carry
//        if ((self.regs.a & 0xF) + (self.regs.e & 0xF)) & 0x10 {
//            self.regs.f |= HALF_CARRY_FLAG;
//        }

        // Carry
        if result > 0xFF {
            self.regs.f |= CARRY_FLAG;
        }

        // save it in the A register
        self.regs.a = (result & 0xFF) as u8;

        self.regs.m = 1;
        self.regs.t = 4;
    }

    // fetches the next operation
    fn fetch_next_op(&mut self) -> u8 {
        let op = self.mmu.read_byte(self.regs.pc);
        self.regs.pc += 1;
        op
    }

    // fetch the operation, decodes it, fetch parameters if required, and executes it
    fn step(&mut self) {
        let operation: u8 = self.fetch_next_op();

        match operation {
            0x00 => { self.nop(); }
            _ => { panic!("Operation not found!!") }
        }

        // add to the clocks
        self.clks.t += self.regs.t as u32;
        self.clks.m += self.regs.m as u32;
    }

    // no operation
    fn nop(&mut self) {
        self.regs.m = 1;
        self.regs.t = 4;
    }

    // push b and c on the stack
    fn pushbc(&mut self) {
        self.regs.sp -= 1;
        self.mmu.write_byte(self.regs.sp, self.regs.b);
        self.regs.sp -= 1;
        self.mmu.write_byte(self.regs.sp, self.regs.c);

        self.regs.m = 3; self.regs.t = 12;
    }

    // pop b and c from the stack
    fn popbc(&mut self) {
        self.regs.c = self.mmu.read_byte(self.regs.sp);
        self.regs.sp += 1;
        self.regs.b = self.mmu.read_byte(self.regs.sp);
        self.regs.sp += 1;

        self.regs.m = 3; self.regs.t = 12;
    }

    // read a word from an absolute location into A
    fn ldamm(&mut self) {
        let addr: u16 = self.mmu.read_word(self.regs.pc);
        self.regs.pc += 2;
        self.regs.a = self.mmu.read_byte(addr);

        self.regs.m = 4; self.regs.t = 16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mem::MMU;

    #[test]
    fn cpu_inizialization() {
        let CPU { clks, regs, .. } = CPU::new(MMU::new());

        assert_eq!(clks.m, 0);
        assert_eq!(clks.t, 0);

        assert_eq!(regs.a, 0);
        assert_eq!(regs.b, 0);
        assert_eq!(regs.c, 0);
        assert_eq!(regs.d, 0);
        assert_eq!(regs.e, 0);
        assert_eq!(regs.h, 0);
        assert_eq!(regs.l, 0);
        assert_eq!(regs.f, 0);
        assert_eq!(regs.pc, 0);
        assert_eq!(regs.sp, 20);
        assert_eq!(regs.m, 0);
        assert_eq!(regs.t, 0);
    }

    #[test]
    fn op_nop() {
        let mut cpu = CPU::new(MMU::new());

        cpu.nop();

        assert_eq!(cpu.regs.m, 1);
        assert_eq!(cpu.regs.t, 4);
    }

    #[test]
    fn op_addr_e() {
        let mut cpu = CPU::new(MMU::new());

        cpu.regs.e = 0xFF;

        cpu.addr_e();

        assert_eq!(cpu.regs.a, 0xFF);
        assert_eq!(cpu.regs.f, 0);

        assert_eq!(cpu.regs.m, 1);
        assert_eq!(cpu.regs.t, 4);
    }

    #[test]
    fn op_addr_e_carry() {
        let mut cpu = CPU::new(MMU::new());

        cpu.regs.a = 0x01;
        cpu.regs.e = 0xFF;

        cpu.addr_e();

        assert_eq!(cpu.regs.a, 0);
        assert_eq!(cpu.regs.f, ZERO_FLAG | CARRY_FLAG);

        assert_eq!(cpu.regs.m, 1);
        assert_eq!(cpu.regs.t, 4);
    }


    #[test]
    fn op_pushbc() {
        let mut cpu = CPU::new(MMU::new());

        cpu.regs.b = 1;
        cpu.regs.c = 2;

        cpu.pushbc();

        cpu.regs.b = 0;
        cpu.regs.c = 0;

        cpu.popbc();

        assert_eq!(cpu.regs.b, 1);
        assert_eq!(cpu.regs.c, 2);
    }

    #[test]
    fn test_step_nop() {
        /// Test that the cpu test method fetches the instruction and executes
        let mut cpu = CPU::new(MMU::new());

        // set next op as nop
        cpu.mmu.write_byte(cpu.regs.pc, 0);

        cpu.step();

        // assert that nop has been executed
        assert_eq!(cpu.regs.m, 1);
        assert_eq!(cpu.regs.t, 4);
    }
}