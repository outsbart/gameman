use std::collections::HashMap;

use mem::Memory;
use ops::{Ops, Operation};

// Flags bit poisition in the F register
const ZERO_FLAG: u8 = 7;
const OPERATION_FLAG: u8 = 6;
const HALF_CARRY_FLAG: u8 = 5;
const CARRY_FLAG: u8 = 4;

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

//const REG_A: String = format!("A");
//const REG_F: String = format!("F");
//const REG_B: String = format!("B");
//const REG_C: String = format!("C");
//const REG_D: String = format!("D");
//const REG_E: String = format!("E");
//const REG_H: String = format!("H");
//const REG_L: String = format!("L");
//const REG_SP: String = format!("SP");
//const REG_S: String = format!("S");
//const REG_PSP: String = format!("PSP");
//const REG_PC: String = format!("PC");
//const REG_CPC: String = format!("CPC");
//const REG_M: String = format!("M");
//const REG_T: String = format!("T");

struct Clocks {
    m: u32, t: u32  // TODO: check if i32 is the right type
}

impl Clocks {
    fn new() -> Clocks {
        Clocks { m: 0, t: 0 }
    }
}

struct Regs {
  //regs_map: HashMap<String, u16>,
    regs: [u8; 14]
}

impl Regs {
    fn new() -> Regs {
//        let mut map = HashMap::new();
//        map.insert(REG_A, 0);
//        map.insert(REG_F, 1);
//        map.insert(REG_B, 2);
//        map.insert(REG_C, 3);
//        map.insert(REG_D, 4);
//        map.insert(REG_E, 5);
//        map.insert(REG_H, 6);
//        map.insert(REG_L, 7);
//        map.insert(REG_SP, 8);
//        map.insert(REG_S, 8);
//        map.insert(REG_PSP, 9);
//        map.insert(REG_PC, 10);
//        map.insert(REG_CPC, 11);
//        map.insert(REG_M, 12);
//        map.insert(REG_T, 13);

        Regs {
            //regs_map: map,
            regs: [0; 14]
        }
    }

    pub fn get_flags(&mut self) -> (bool, bool, bool, bool) {
        let f = self.read_byte(REG_F) as u16;
        (get_bit(ZERO_FLAG, f), get_bit(OPERATION_FLAG, f), get_bit(HALF_CARRY_FLAG, f), get_bit(CARRY_FLAG, f))
    }

    pub fn set_flags(&mut self, z: bool, o: bool, h: bool, c: bool) {
        let value = ((z as u8) << ZERO_FLAG) | ((o as u8) << OPERATION_FLAG) | ((h as u8) << HALF_CARRY_FLAG) | ((c as u8) << CARRY_FLAG);
        self.write_byte(REG_F, value)
    }
}


fn get_bit(pos: u8, value: u16) -> bool {
    value & (1u16 << pos) != 0
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

    // fetches the next byte from the ram
    fn fetch_next_byte(&mut self) -> u8 {
        let byte = self.mmu.read_byte(self.regs.read_word(REG_PC));
        let pc_value = self.regs.read_word(REG_PC);
        self.regs.write_word(REG_PC, pc_value + 1);
        byte
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
        let op: Operation = ops.fetch_operation(self);

        println!("0x{:x}\t{}\t{}\t{:?}\t{:?}", op.code_as_u8(), op.into, op.mnemonic, op.operand1, op.operand2);

        self.execute(&op);

        // add to the clocks
        self.clks.t += self.regs.read_byte(REG_T) as u32;
        self.clks.m += self.regs.read_byte(REG_M) as u32;
    }

    fn registry_name_to_index(&mut self, registry: &str) -> u16 {
        match registry {
            "A" => { 0 }, "F" => { 1 }, "B" => { 2 }, "C" => { 3 }, "D" => { 4 }
            "E" => { 5 }, "H" => { 6 }, "HL" => { 6 }, "L" => { 7 }, "SP" => { 8 }, "S" => { 8 }
            "PSP" => { 9 }, "PC" => { 10 }, "CPC" => { 11 }, "M" => { 12 }, "T" => { 13 }
            _ => { panic!("What kind of register is {}??", registry) }
        }
    }

    pub fn get_registry_value(&mut self, registry: &str) -> u16 {
        let index: u16 = self.registry_name_to_index(registry);
        match registry.len() {
            1 => { self.regs.read_byte(index) as u16 }
            _ => { self.regs.read_word(index) }
        }
    }

    pub fn set_registry_value(&mut self, registry: &str, value: u16) {
        let index: u16 = self.registry_name_to_index(registry);
        match registry.len() {
            1 => { self.regs.write_byte(index, value as u8) }
            _ => { self.regs.write_word(index, value) }
        }
    }

    pub fn store_result(&mut self, into: &str, value: u16) {
        println!("Storing into {} value {}", into, value);
        match into.as_ref() {
            "(BC)"|"(DE)"|"(HL)"|"(PC)"|"(SP)" => {
                let reg = into[1..into.len()-1].as_ref();
                let addr = self.get_registry_value(reg);
                self.mmu.write_word(addr, value);
            }
            "BC"|"DE"|"HL"|"PC"|"SP"|
            "A"|"B"|"C"|"D"|"E"|"H"|"L" => { self.set_registry_value(into, value) }
            _ => { panic!("cant write to {} yet!!!", into) }
        }
    }

    pub fn get_operand_value(&mut self, operand: &str) -> u16 {
        match operand.as_ref() {
            "(BC)"|"(DE)"|"(HL)"|"(PC)"|"(SP)" => {
                let reg = operand[1..operand.len()-1].as_ref();
                let addr = self.get_registry_value(reg);
                self.mmu.read_word(addr)
            }
            "BC"|"DE"|"HL"|"PC"|"SP"|
            "A"|"B"|"C"|"D"|"E"|"H"|"L" => { self.get_registry_value(operand) }
            "d16" => { self.fetch_next_word() }
            "d8" => { self.fetch_next_byte() as u16 }
            _ => {
                operand.parse::<u16>().expect(format!("cant read {} yet!!!", operand).as_ref())
            }
        }
    }

    pub fn execute(&mut self, op: &Operation) {
        let op1 = match op.operand1 {
            Some (ref x) => { self.get_operand_value(x) }
            None => { 0 }
        };
        let op2 = match op.operand2 {
            Some (ref x) => { self.get_operand_value(x) }
            None => { 0 }
        };
        let mut result: u16 = 0;
        let (mut z, mut o, mut h, mut c) = self.regs.get_flags();

        println!("0x{:x}\t{}\t{:x}\t{:x}", op.code_as_u8(), op.mnemonic, op1, op2);

        match op.mnemonic.as_ref() {
            "NOP" => {},
            "LD" => { result = op1 },
            "LDD" => {
                println!("Implement decrease!");
                result = op1
            },
            "XOR" => { result = op1 ^ op2 },
            "BIT" => {
                z = !get_bit(op1 as u8, op2)
            }
            _ => {
                panic!("0x{:x}\t{} not implemented yet!", op.code_as_u8(), op.mnemonic);
            }
        }

        if op.into != "" {
            self.store_result(op.into.as_ref(), result);
        }

        self.regs.set_flags(z, o, h, c);
        self.regs.write_byte(REG_T, op.cycles_ok);
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
    fn get_flags() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.regs.set_flags(true,false,true,false);
        let (z, o, h, c) = cpu.regs.get_flags();

        assert_eq!(z, true);
        assert_eq!(o, false);
        assert_eq!(h, true);
        assert_eq!(c, false);
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