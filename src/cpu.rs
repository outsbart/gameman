use mem::Memory;
use ops::{Ops, Operation};

// Flags poisition in the F register
const ZERO_FLAG: u8 = 0x80;
const OPERATION_FLAG: u8 = 0x40;
const HALF_CARRY_FLAG: u8 = 0x20;
const CARRY_FLAG: u8 = 0x10;

// Registers are saved inside an array so that i can use consecutive indexes to access
// those registers that can be accessed together like B and C
// eg: read B with REG_B index, and BC with REG_B and REG_B + 1 indexes
const REG_A: u16 = 0;
const REG_F: u16 = 1;
const REG_B: u16 = 2;
const REG_C: u16 = 3;
const REG_D: u16 = 4;
const REG_E: u16 = 5;
const REG_H: u16 = 6;
const REG_L: u16 = 7;
const REG_SP: u16 = 8;
const REG_S: u16 = 8;
const REG_PSP: u16 = 9;
const REG_PC: u16 = 10;
const REG_CPC: u16 = 11;
const REG_M: u16 = 12;
const REG_T: u16 = 13;

struct Clocks {
    m: u32, t: u32  // TODO: check if i32 is the right type
}

impl Clocks {
    fn new() -> Clocks {
        Clocks { m: 0, t: 0 }
    }
}

struct Regs { regs: [u8; 14] }

impl Regs {
    fn new() -> Regs {
        Regs { regs: [0; 14] }
    }
}

pub trait ByteStream {
    fn read_byte(&mut self) -> u8;
    fn read_word(&mut self) -> u16;
}

impl Memory for Regs {
    fn read_byte(&mut self, addr: u16) -> u8 { self.regs[addr as usize] }
    fn write_byte(&mut self, addr: u16, byte: u8) { self.regs[addr as usize] = byte; }
}

pub struct CPU<M: Memory> {
    clks: Clocks,
    regs: Regs,
    mmu: M
}

impl<M: Memory> ByteStream for CPU<M> {
    fn read_byte(&mut self) -> u8 {
        self.fetch_next_byte()
    }
    fn read_word(&mut self) -> u16 {
        self.fetch_next_word()
    }
}

impl<M: Memory> CPU<M> {
    pub fn new(mmu: M) -> CPU<M> {
        CPU { clks: Clocks::new(), regs: Regs::new(), mmu }
    }

    // operations

    // adds E to A
//    fn addr_e(&mut self) {
//        let result = self.regs.read_byte(REG_A) as u32 + self.regs.read_byte(REG_E) as u32;
//
//        self.regs.write_byte(REG_F,0); // reset the flags!
//
//        // Zero
//        if (result & 0xFF) == 0 {
//            self.regs[REG_F] |= ZERO_FLAG; // if result is 0 set the first bit to 1
//        }
//
//        // Half Carry
////        if ((self.regs.a & 0xF) + (self.regs.e & 0xF)) & 0x10 {
////            self.regs.f |= HALF_CARRY_FLAG;
////        }
//
//        // Carry
//        if result > 0xFF {
//            self.regs[REG_F] |= CARRY_FLAG;
//        }
//
//        // save it in the A register
//        self.regs[REG_A] = (result & 0xFF) as u8;
//
//        self.regs[REG_M] = 1;
//        self.regs[REG_T] = 4;
//    }

    // fetches the next byte from the ram
    fn fetch_next_byte(&mut self) -> u8 {
        let op = self.mmu.read_byte(self.regs.read_word(REG_PC));
        let pc_value = self.regs.read_word(REG_PC);
        self.regs.write_word(REG_PC, pc_value + 1);
        op
    }

    // fetches the next word from the ram
    fn fetch_next_word(&mut self) -> u16 {
        let word = self.mmu.read_word(self.regs.read_word(REG_PC));
        let pc_value = self.regs.read_word(REG_PC);
        self.regs.write_word(REG_PC, pc_value + 2);
        word
    }

    // fetch the operation, decodes it, fetch parameters if required, and executes it
    pub fn step(&mut self) {
        let mut ops = Ops::new();  // todo: make it an attribute...
        let op: &Operation = ops.fetch_operation(self);

        println!("0x{:x}\t{}\t{:?}\t{:?}", op.code_as_u8(), op.mnemonic, op.operand1, op.operand2);
        self.execute(op);

        // add to the clocks
        self.clks.t += self.regs.read_byte(REG_T) as u32;
        self.clks.m += self.regs.read_byte(REG_M) as u32;
    }

    pub fn execute(&mut self, op: &Operation) {
        match op.mnemonic.as_ref() {
            "NOP" => {},
            "LD" => {

            },
            _ => {
                panic!("0x{:x}\t{} not implemented yet!", op.code_as_u8(), op.mnemonic);
            }
        }

        self.regs.write_byte(REG_T, op.cycles_ok);
    }

    // no operation
    fn nop(&mut self) {
        self.regs.write_byte(REG_M, 1);
        self.regs.write_byte(REG_T, 4);
    }

//    // read a word from an absolute location into A
//    fn ldamm(&mut self) {
//        let addr: u16 = self.mmu.read_word(self.regs.pc); // TODO FIX
//        self.regs[Reg.PC] += 2;  //TODO FIX
//        self.regs[Reg.A] = self.mmu.read_byte(addr);
//
//        self.regs[Reg.M] = 4; self.regs[Reg.T] = 16;
//    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyMMU {
        values: [u8; 65536]
    }

    impl DummyMMU {
        fn new() -> DummyMMU { DummyMMU{ values: [0; 65536] } }
        fn with(values: [u8; 65536]) -> DummyMMU { DummyMMU{ values } }
    }

    impl Memory for DummyMMU {
        fn read_byte(&mut self, addr: u16) -> u8 {
            self.values[addr as usize]
        }
        fn write_byte(&mut self, addr: u16, byte: u8) {
            self.values[addr as usize] = byte;
        }
    }

    #[test]
    fn cpu_inizialization() {
        let CPU { clks, mut regs, .. } = CPU::new(DummyMMU::new());

        assert_eq!(clks.m, 0);
        assert_eq!(clks.t, 0);

        assert_eq!(regs.read_byte(REG_A), 0);
        assert_eq!(regs.read_byte(REG_B), 0);
        assert_eq!(regs.read_byte(REG_C), 0);
        assert_eq!(regs.read_byte(REG_D), 0);
        assert_eq!(regs.read_byte(REG_E), 0);
        assert_eq!(regs.read_byte(REG_H), 0);
        assert_eq!(regs.read_byte(REG_L), 0);
        assert_eq!(regs.read_byte(REG_F), 0);
        assert_eq!(regs.read_word(REG_PC), 0);
        assert_eq!(regs.read_word(REG_SP), 0);
        assert_eq!(regs.read_byte(REG_M), 0);
        assert_eq!(regs.read_byte(REG_T), 0);
    }

    #[test]
    fn op_nop() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.nop();

        assert_eq!(cpu.regs.read_byte(REG_M), 1);
        assert_eq!(cpu.regs.read_byte(REG_T), 4);
    }

//    #[test]
//    fn op_addr_e() {
//        let mut cpu = CPU::new(DummyMMU::new());
//
//        cpu.regs[Reg.E] = 0xFF;
//
//        cpu.addr_e();
//
//        assert_eq!(cpu.regs[Reg.A], 0xFF);
//        assert_eq!(cpu.regs[Reg.F], 0);
//
//        assert_eq!(cpu.regs[Reg.M], 1);
//        assert_eq!(cpu.regs[Reg.T], 4);
//    }
//
//    #[test]
//    fn op_addr_e_carry() {
//        let mut cpu = CPU::new(DummyMMU::new());
//
//        cpu.regs[Reg.A] = 0x01;
//        cpu.regs[Reg.E] = 0xFF;
//
//        cpu.addr_e();
//
//        assert_eq!(cpu.regs[Reg.A], 0);
//        assert_eq!(cpu.regs[Reg.F], ZERO_FLAG | CARRY_FLAG);
//
//        assert_eq!(cpu.regs[Reg.M], 1);
//        assert_eq!(cpu.regs[Reg.T], 4);
//    }
}