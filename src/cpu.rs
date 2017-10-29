use mem::Memory;
use ops::{Ops, Operation};
use utils::{u8_to_i8, u16_to_i16, rotate_left};

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


struct Clocks {
    m: u32, t: u32  // TODO: check if i32 is the right type
}

impl Clocks {
    fn new() -> Self {
        Clocks { m: 0, t: 0 }
    }
}

struct Regs { regs: [u8; 14] }

impl Regs {
    fn new() -> Regs { Regs { regs: [0; 14] } }

    pub fn get_flags(&mut self) -> (bool, bool, bool, bool) {
        let f = u16::from(self.read_byte(REG_F));
        (get_bit(ZERO_FLAG, f), get_bit(OPERATION_FLAG, f), get_bit(HALF_CARRY_FLAG, f), get_bit(CARRY_FLAG, f))
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        let value = ((z as u8) << ZERO_FLAG) | ((n as u8) << OPERATION_FLAG) | ((h as u8) << HALF_CARRY_FLAG) | ((c as u8) << CARRY_FLAG);
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
    mmu: M,
    ops: Ops
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
        let mut cpu = CPU { clks: Clocks::new(), regs: Regs::new(), mmu, ops: Ops::new() };
        cpu.reset();
        cpu
    }

    // initalize
    fn reset(&mut self) {
        self.set_registry_value("SP", 0xFFFE);
        //TODO: set all registry to zero. RAM as well
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
        let mut prefixed = false;
        let mut byte = self.read_byte();

        if byte == 0xcb {
            byte = self.read_byte();
            prefixed = true;
        }

        let op: Operation = self.ops.fetch_operation(byte, prefixed);

        println!("0x{:x}\t0x{:x}\t{}\t{:?}\t{:?}", self.get_registry_value("PC"), op.code_as_u8(), op.mnemonic, op.operand1, op.operand2);

        self.execute(&op);

        // add to the clocks
        self.clks.t += u32::from(self.regs.read_byte(REG_T));
        self.clks.m += u32::from(self.regs.read_byte(REG_M));
    }

    fn registry_name_to_index(&mut self, registry: &str) -> u16 {
        match registry {
            "A" => { 0 }, "F" => { 1 }, "B"|"BC" => { 2 }, "C" => { 3 }, "D"|"DE" => { 4 }
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
            "(C)" => {
                let reg = into[1..into.len()-1].as_ref();
                let addr = self.get_registry_value(reg) + 0xFF00;
                self.mmu.write_word(addr, value);
            }
            "(a8)" => {
                let addr = u16::from(self.fetch_next_byte() + 0xFF00);
                self.mmu.write_word(addr, value);
            }
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
            "d16"|"a16" => { self.fetch_next_word() }
            "d8"|"r8" => { self.fetch_next_byte() as u16 }
            "NZ" => { !self.regs.get_flags().0 as u16 }
            _ => {
                operand.parse::<u16>().expect(format!("cant read {} yet!!!", operand).as_ref())
            }
        }
    }

    pub fn push(&mut self, value: u16) {
        let sp = self.get_registry_value("SP");
        self.set_registry_value("SP", sp-2);
        self.store_result("(SP)", value);
    }

    pub fn pop(&mut self) -> u16 {
        let sp = self.get_registry_value("SP");
        let value = self.mmu.read_word(sp);
        self.set_registry_value("SP",sp+2);
        value
    }

    pub fn execute(&mut self, op: &Operation) {
        let mut do_action = true;

        let op1 = match op.operand1 {
            Some (ref x) => { self.get_operand_value(x) }
            None => { 0 }
        };
        let op2 = match op.operand2 {
            Some (ref x) => { self.get_operand_value(x) }
            None => { 0 }
        };
        let op3 = match op.operand3 {
            Some (ref x) => { self.get_operand_value(x) }
            None => { 0 }
        };

        let mut result: u16 = 0;

        let (mut z, mut n, mut h, mut c) = self.regs.get_flags();

        match op.flag_z.unwrap_or(' ') {
            '0' => { z = false },
            '1' => { z = true },
            _ => {}
        };
        match op.flag_n.unwrap_or(' ') {
            '0' => { n = false },
            '1' => { n = true },
            _ => {}
        };
        match op.flag_h.unwrap_or(' ') {
            '0' => { h = false },
            '1' => { h = true },
            _ => {}
        };
        match op.flag_c.unwrap_or(' ') {
            '0' => { c = false },
            '1' => { c = true },
            _ => {}
        }

        println!("\t0x{:x}\t{}\t{:x}\t{:x}", op.code_as_u8(), op.mnemonic, op1, op2);

        match op.mnemonic.as_ref() {
            "NOP" => {},
            "LD"|"LDD"|"LDH" => { result = op1 },
            "XOR" => { result = op1 ^ op2 },
            "BIT" => { z = !get_bit(op1 as u8, op2) }
            "INC" => {
                result = op1 + 1;
                z = result == 0;
                n = false;
                h = ((op1 & 0xF) + 1) & 0x10 != 0; // TODO: should be ok
            }
            "JR" => {
                if op3 == 0 {
                    do_action = false;
                }
                //TODO: handle possible overflow
                result = (u16_to_i16(op1)+1 + u8_to_i8(op2 as u8) as i16) as u16;
            }
            "CALL" => {
                if op3 == 0 {
                    do_action = false;
                }
                else {
                    let value = self.get_registry_value("PC");
                    self.push(value);
                    result = op1;
                }
            }
            "PUSH" => { self.push(op1) }
            "POP" => { result = self.pop() }
            "RL"|"RLA" => { result = rotate_left(op1 as u8) }
//            "RES" => {
//                result = !(1u16<<op1) ^ op2;
//            }
            _ => {
                panic!("0x{:x}\t{} not implemented yet!", op.code_as_u8(), op.mnemonic);
            }
        }

        if do_action && op.into != "" {
            self.store_result(op.into.as_ref(), result);
        }

        // postaction
        match op.mnemonic.as_ref() {
            "LDD" => {
                let reg: &str = op.into[1..op.into.len() - 1].as_ref();
                let value = self.get_registry_value(reg);
                self.store_result(reg, value - 1);
            }
            _ => {}
        }

        self.regs.set_flags(z, n, h, c);
        self.regs.write_byte(REG_T, op.cycles_ok);
    }
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
        let (z, n, h, c) = cpu.regs.get_flags();

        assert_eq!(z, true);
        assert_eq!(n, false);
        assert_eq!(h, true);
        assert_eq!(c, false);
    }

    #[test]
    fn test_jr_positive() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.set_registry_value("PC", 500);
        cpu.mmu.values[500] = 0x18;
        cpu.mmu.values[501] = 0b0000_0010; // jump by 2

        cpu.step();

        assert_eq!(cpu.get_registry_value("PC"), 504);
    }

    #[test]
    fn test_jr_negative() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.set_registry_value("PC", 500);
        cpu.mmu.values[500] = 0x18;
        cpu.mmu.values[501] = 0b1111_1110; // jump by -2

        cpu.step();


        //TODO: MAKE SURE IT SHOULD GO BACK BY 2 CONSIDERING THE OPERAND READING
        assert_eq!(cpu.get_registry_value("PC"), 500);
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