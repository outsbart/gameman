use crate::mem::Memory;
use crate::ops::{fetch_operation, Operation};
use crate::utils::add_bytes;
use crate::utils::add_word_with_signed;
use crate::utils::add_words;
use crate::utils::parse_hex;
use crate::utils::reset_bit;
use crate::utils::set_bit;
use crate::utils::sub_bytes;
use crate::utils::swap_nibbles;

pub const CPU_FREQ: usize = 4194304;  // cpu frequency, in hz

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

pub struct Clocks {
    // todo: remove pub
    m: u32,
    pub t: u32, // TODO: remove pub
}

impl Clocks {
    fn new() -> Self {
        Clocks { m: 0, t: 0 }
    }
}

struct Regs {
    regs: [u8; 14],
}

impl Regs {
    fn new() -> Regs {
        Regs { regs: [0; 14] }
    }

    pub fn get_flags(&mut self) -> (bool, bool, bool, bool) {
        let f = u16::from(self.read_byte(REG_F));
        (
            is_bit_set(ZERO_FLAG, f),
            is_bit_set(OPERATION_FLAG, f),
            is_bit_set(HALF_CARRY_FLAG, f),
            is_bit_set(CARRY_FLAG, f),
        )
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        let value = ((z as u8) << ZERO_FLAG)
            | ((n as u8) << OPERATION_FLAG)
            | ((h as u8) << HALF_CARRY_FLAG)
            | ((c as u8) << CARRY_FLAG);
        self.write_byte(REG_F, value)
    }
}

pub fn is_bit_set(pos: u8, value: u16) -> bool {
    value & (1u16 << pos) != 0
}

pub trait ByteStream {
    fn read_byte(&mut self) -> u8;
    fn read_word(&mut self) -> u16;
}

impl Memory for Regs {
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.regs[addr as usize]
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        // The F register lower nibble is always 0, you cant overwrite it.
        self.regs[addr as usize] = if addr != REG_F { byte } else { byte & 0xF0 };
    }
    fn read_word(&mut self, addr: u16) -> u16 {
        (self.read_byte(addr + 1) as u16) | ((self.read_byte(addr) as u16) << 8)
    }
    fn write_word(&mut self, addr: u16, word: u16) -> () {
        self.write_byte(addr + 1, (word & 0x00FF) as u8);
        self.write_byte(addr, ((word & 0xFF00) >> 8) as u8);
    }
}

pub struct CPU<M: Memory> {
    pub clks: Clocks,
    regs: Regs,
    pub mmu: M,
    interrupt_master_enable: bool,
    schedule_interrupt_enable: bool, // if set to true, next step interrupt_master_enable will be set to 1
    stopped: bool,
    halted: bool, // used for HALT
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
        let mut cpu = CPU {
            clks: Clocks::new(),
            regs: Regs::new(),
            mmu,
            interrupt_master_enable: false,
            schedule_interrupt_enable: false,
            stopped: false,
            halted: false,
        };
        cpu.reset();
        cpu
    }

    // initalize
    fn reset(&mut self) {
        self.set_registry_value("SP", 0xFFFE);
        self.set_registry_value("PC", 0x100);
        self.interrupt_master_enable = true;
        //TODO: set all registry to zero. RAM as well
    }

    // fetches the next byte from the ram
    fn fetch_next_byte(&mut self) -> u8 {
        let byte = self.mmu.read_byte(self.regs.read_word(REG_PC));
        let pc_value = self.regs.read_word(REG_PC);
        self.regs.write_word(REG_PC, pc_value.wrapping_add(1));
        byte
    }

    // fetches the next word from the ram
    fn fetch_next_word(&mut self) -> u16 {
        let word = self.mmu.read_word(self.regs.read_word(REG_PC));
        let pc_value = self.regs.read_word(REG_PC);
        self.regs.write_word(REG_PC, pc_value.wrapping_add(2));
        word
    }

    // fetch the operation, decodes it, fetch parameters if required and executes it.
    // returns the address of the executed instruction
    pub fn step(&mut self) -> (u16, u8) {
        let line_number = self.get_registry_value("PC");

        let mut cycles_this_step: u8 = 0;

        if !self.halted {
            let mut prefixed = false;
            let mut byte = self.read_byte();

            if byte == 0xcb {
                byte = self.read_byte();

                prefixed = true;
            }

            let op: &Operation = fetch_operation(byte, prefixed);

            info!(
                "0x{:x}\t0x{:x}\t{}\t{:?}\t{:?}",
                line_number,
                op.code_as_u8(),
                op.mnemonic,
                op.operand1,
                op.operand2
            );

            if self.schedule_interrupt_enable {
                self.interrupt_master_enable = true;
                self.schedule_interrupt_enable = false;
            }

            if false {
                self.execute_old(op);
            } else {
                // lets use temporarily M to see if the condition failed
                self.regs.write_byte(REG_M, 0);

                self.execute(byte, prefixed);

                if self.regs.read_byte(REG_M) == 0 {
                    self.regs.write_byte(REG_T, op.cycles_ok)
                } else {
                    self.regs.write_byte(REG_T, op.cycles_no.expect("wat?"))
                }
            }
        } else {
            self.regs.write_byte(REG_T, 4);
        }

        cycles_this_step += self.regs.read_byte(REG_T);

        self.tick_timers();

        self.handle_interrupts();

        cycles_this_step += self.regs.read_byte(REG_T);

        self.tick_timers();

        (line_number, cycles_this_step)
    }

    pub fn execute_old(&mut self, op: &Operation) {
        self.regs.write_byte(REG_T, 4);

        if self.schedule_interrupt_enable {
            self.interrupt_master_enable = true;
            self.schedule_interrupt_enable = false;
        }

        let mut op2_is_signed: bool = false;

        // care, some of this might send PC forward
        let op1 = match op.operand1 {
            Some(ref x) => self.get_operand_value(x),
            None => 0,
        };
        let op2 = match op.operand2 {
            Some(ref x) => {
                op2_is_signed = x == "r8";
                self.get_operand_value(x)
            }
            None => 0,
        };
        let condition = match op.condition {
            Some(ref x) => self.get_operand_value(x),
            None => 1,
        };

        let cycles = op.cycles_ok;

        // early stop
        // dont perform the operation if condition == 0
        if condition == 0 {
            info!(
                "operation 0x{:x} {} skipped cause condition {}",
                op.code_as_u8(),
                op.mnemonic,
                condition
            );
            let cycles_no = op
                .cycles_no
                .expect("Operation skipped but cycles_no not set.");

            self.regs.write_byte(REG_T, cycles_no);
            return;
        }

        let result_is_byte: bool = match op.result_is_byte {
            Some(_x) => true,
            None => false,
        };

        let mut result: u16 = 1;
        let (prev_z, prev_n, prev_h, prev_c) = self.regs.get_flags();
        let mut new_carry = prev_c;
        let mut new_halfcarry = prev_h;

        info!(
            "istruzione\t0x{:x}\t{}\top1={:x}\top2={:x}\tinto={}",
            op.code_as_u8(),
            op.mnemonic,
            op1,
            op2,
            op.into
        );

        match op.mnemonic.as_ref() {
            "NOP" => {}
            "DI" => {
                self.interrupt_master_enable = false;
            },
            "EI" => self.schedule_interrupt_enable = true,
            "STOP" => self.stopped = true,
            "LD" | "LDD" | "LDH" | "LDI" | "JP" => {
                result = op1;
                if op2_is_signed {
                    let (x, y, z) = add_word_with_signed(op1, op2, 0);
                    result = x;
                    new_carry = y;
                    new_halfcarry = z;
                }
            }
            "AND" => result = op1 & op2,
            "OR" => result = op1 | op2,
            "XOR" => {
                result = op1 ^ op2;
            }
            "CPL" => {
                result = !op1;
            }
            "BIT" => {
                result = is_bit_set(op1 as u8, op2) as u16;
            }
            "PUSH" => self.push(op1),
            "POP" | "RET" => result = self.pop(),
            "RETI" => {
                result = self.pop();
                self.interrupt_master_enable = true;
            }
            "JR" => {
                result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;
            }
            "CALL" | "RST" => {
                let value = self.get_registry_value("PC");
                self.push(value);
                result = op1;
            }
            "DEC" | "SUB" | "SBC" | "CP" => {
                let third_param: u16 = if op.mnemonic == String::from("SBC") {
                    u16::from(prev_c)
                } else {
                    0
                };
                let (x, y, z) = sub_bytes(op1, op2, third_param);
                result = x;
                new_carry = y;
                new_halfcarry = z;
            }
            "INC" | "ADD" | "ADC" => {
                let sum_func = if op2_is_signed {
                    add_word_with_signed
                } else {
                    if result_is_byte {
                        add_bytes
                    } else {
                        add_words
                    }
                };
                let third_param: u16 = if op.mnemonic == "ADC" {
                    u16::from(prev_c)
                } else {
                    0
                };
                let (x, y, z) = sum_func(op1, op2, third_param);
                result = x;
                new_carry = y;
                new_halfcarry = z;
            }
            "RL" | "RLA" => {
                result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
                new_carry = (op1 & 0x80) != 0;
            }
            "RLC" => {
                result = (op1 << 1) | (op1 >> 7);
                new_carry = (op1 & 0x80) != 0
            }
            "RRC" => {
                result = (op1 >> 1) | (op1 << 7);
                new_carry = (op1 & 1) != 0
            }
            "SLA" => {
                result = ((op1 as u8) << 1) as u16;
                new_carry = (op1 & 0x80) != 0;
            }
            "SRA" => {
                result = (op1 >> 1) | (op1 & 0x80);
                new_carry = (op1 & 1) != 0;
            }
            "SCF" => {
                new_carry = true;
            }
            "CCF" => {
                new_carry = !prev_c;
            }
            "RLCA" => {
                new_carry = (op1 & 0x80) != 0;
                result = ((op1 as u8) << 1 | u8::from(new_carry)) as u16;
            }
            "RRCA" => {
                new_carry = (op1 & 1) != 0;
                result = ((op1 as u8) >> 1 | (u8::from(new_carry) << 7)) as u16;
            }
            "SRL" => {
                result = op1 >> 1;
                new_carry = (op1 & 1) != 0;
            }
            "RR" | "RRA" => {
                result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
                new_carry = (op1 & 1) != 0;
            }
            "DAA" => {
                let mut adjust = 0;

                if prev_h {
                    adjust |= 0x06;
                }

                if prev_c {
                    adjust |= 0x60;
                    new_carry = true;
                }

                result = if prev_n {
                    op1.wrapping_sub(adjust)
                } else {
                    if op1 & 0x0F > 0x09 {
                        adjust |= 0x06;
                    }

                    if op1 > 0x99 {
                        adjust |= 0x60;
                        new_carry = true;
                    }

                    op1.wrapping_add(adjust)
                };
            }
            "SWAP" => result = swap_nibbles(op1 as u8),
            "RES" => {
                result = reset_bit(op1 as u8, op2 as u8);
            }
            "SET" => {
                result = set_bit(op1 as u8, op2 as u8);
            }
            "HALT" => {
                self.halted = true;
            } // todo: implement halt bug
            _ => {
                panic!(
                    "0x{:x}\t{} not implemented yet!",
                    op.code_as_u8(),
                    op.mnemonic
                );
            }
        }

        if result_is_byte {
            result = (result as u8) as u16;
        }

        // set the flags
        self.regs.set_flags(
            match op.flag_z.unwrap_or(' ') {
                '0' => false,
                '1' => true,
                'Z' => result == 0,
                _ => prev_z,
            },
            match op.flag_n.unwrap_or(' ') {
                '0' => false,
                '1' => true,
                _ => prev_n,
            },
            match op.flag_h.unwrap_or(' ') {
                '0' => false,
                '1' => true,
                'H' => new_halfcarry,
                _ => prev_h,
            },
            match op.flag_c.unwrap_or(' ') {
                '0' => false,
                '1' => true,
                'C' => new_carry,
                _ => prev_c,
            },
        );

        // store the operation result
        if op.into != String::from("") {
            self.store_result(op.into.as_ref(), result, result_is_byte);
        }

        // perform postactions if necessary
        match op.mnemonic.as_ref() {
            // care: maybe this should be a PREaction
            "LDD" => {
                let reg: &str = "HL";
                let value = self.get_registry_value(reg);
                self.store_result(reg, value.wrapping_sub(1), false);
            }
            "LDI" => {
                let reg: &str = "HL";
                let value = self.get_registry_value(reg);
                self.store_result(reg, value.wrapping_add(1), false);
            }
            _ => {}
        }

        self.regs.write_byte(REG_T, cycles);
    }

    fn registry_name_to_index(&mut self, registry: &str) -> u16 {
        match registry {
            "A" | "AF" => 0,
            "F" => 1,
            "B" | "BC" => 2,
            "C" => 3,
            "D" | "DE" => 4,
            "E" => 5,
            "H" => 6,
            "HL" => 6,
            "L" => 7,
            "SP" => 8,
            "S" => 8,
            "PSP" => 9,
            "PC" => 10,
            "CPC" => 11,
            "M" => 12,
            "T" => 13,
            _ => panic!("What kind of register is {}??", registry),
        }
    }

    pub fn get_registry_value(&mut self, registry: &str) -> u16 {
        let index: u16 = self.registry_name_to_index(registry);
        match registry.len() {
            1 => self.regs.read_byte(index) as u16,
            _ => self.regs.read_word(index),
        }
    }

    pub fn set_registry_value(&mut self, registry: &str, value: u16) {
        let index: u16 = self.registry_name_to_index(registry);
        match registry.len() {
            1 => self.regs.write_byte(index, value as u8),
            _ => self.regs.write_word(index, value),
        }
    }

    pub fn store_result(&mut self, into: &str, value: u16, is_byte: bool) {
        info!("Storing into {} value 0x{:x}", into, value);
        let addr: u16 = match into.as_ref() {
            "BC" | "DE" | "HL" | "PC" | "SP" | "AF" | "A" | "B" | "C" | "D" | "E" | "H" | "L" => {
                return self.set_registry_value(into, value);
            }
            "(BC)" | "(DE)" | "(HL)" | "(PC)" | "(SP)" => {
                let reg = into[1..into.len() - 1].as_ref();
                self.get_registry_value(reg)
            }
            "(C)" => {
                let reg = into[1..into.len() - 1].as_ref();
                self.get_registry_value(reg) + 0xFF00
            }
            "(a8)" => u16::from(self.fetch_next_byte()) + 0xFF00,
            "(a16)" => self.fetch_next_word(),
            _ => panic!("cant write to {} yet!!!", into),
        };
        if is_byte {
            self.mmu.write_byte(addr, value as u8)
        } else {
            self.mmu.write_word(addr, value)
        }
    }

    pub fn get_operand_value(&mut self, operand: &str) -> u16 {
        match operand.as_ref() {
            "(BC)" | "(DE)" | "(HL)" | "(PC)" | "(SP)" => {
                let reg = operand[1..operand.len() - 1].as_ref();
                let addr = self.get_registry_value(reg);
                self.mmu.read_byte(addr) as u16
            }
            "BC" | "DE" | "HL" | "PC" | "SP" | "AF" | "A" | "B" | "C" | "D" | "E" | "H" | "L" => {
                self.get_registry_value(operand)
            }
            "(a8)" => {
                let addr = 0xFF00 + u16::from(self.fetch_next_byte());
                let res = u16::from(self.mmu.read_byte(addr));
                //                info!("Reading input from 0x{:x} --> 0b{:b}", addr, res);
                res
            }
            "(C)" => {
                let addr = 0xFF00 + u16::from(self.get_registry_value("C"));
                u16::from(self.mmu.read_byte(addr))
            }
            "(a16)" => {
                let addr = u16::from(self.fetch_next_word());
                self.mmu.read_byte(addr) as u16
            }
            "d16" | "a16" => self.fetch_next_word(),
            "d8" | "r8" => self.fetch_next_byte() as u16,
            "NZ" => !self.regs.get_flags().0 as u16,
            "Z" => self.regs.get_flags().0 as u16,
            "NC" => !self.regs.get_flags().3 as u16,
            "CA" => self.regs.get_flags().3 as u16,
            _ => parse_hex(operand),
        }
    }

    pub fn push(&mut self, value: u16) {
        let sp = self.get_registry_value("SP");
        self.set_registry_value("SP", sp - 2);
        self.store_result("(SP)", value, false);
    }

    pub fn pop(&mut self) -> u16 {
        let sp = self.get_registry_value("SP");
        let value = self.mmu.read_word(sp);
        self.set_registry_value("SP", sp + 2);
        value
    }

    // update timers relative to cpu clock
    // this function might request a timer Interrupt
    fn tick_timers(&mut self) {
        let cycles = self.regs.read_byte(REG_T);

        self.mmu.tick(cycles);
    }

    // return IE & IF
    fn interrupts_to_handle(&mut self) -> u8 {
        let interrupt_enable = self.mmu.read_byte(0xFFFF);
        let interrupt_flags = self.mmu.read_byte(0xFF0F);
        interrupt_enable & interrupt_flags
    }

    fn handle_interrupts(&mut self) {
        let mut interrupt_cycles_t: u8 = 0;
        let interrupts = self.interrupts_to_handle();

        // wake up cpu if there is an interrupt, even if ime = 0
        if interrupts != 0 && self.halted {
            self.halted = false;
        }

        // if we have to handle an interrupt
        if self.interrupt_master_enable && interrupts != 0 {

            // only one interrupt handling at a time
            self.interrupt_master_enable = false;

            // put current instruction on the stack, handle interrupt immediately
            let value = self.get_registry_value("PC");
            self.push(value);

            interrupt_cycles_t = 12;

            let interrupt_flags = self.mmu.read_byte(0xFF0F);

            // vblank
            if (interrupts & 0x1) != 0 {
                // turn interrupt flag off cause we are handling it now
                self.mmu
                    .write_byte(0xFF0F, reset_bit(0, interrupt_flags) as u8);

                self.set_registry_value("PC", 0x0040);
            }

            // lcd status triggers
            else if (interrupts & 0x2) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(1, interrupt_flags) as u8);

                self.set_registry_value("PC", 0x0048);
            }

            // timer
            if (interrupts & 0x4) != 0 {
                println!("Handling timer");

                self.mmu
                    .write_byte(0xFF0F, reset_bit(2, interrupt_flags) as u8);

                self.set_registry_value("PC", 0x0050);
            }

            // serial
            else if (interrupts & 0b1000) != 0 {
                println!("Handling serial");

                self.mmu
                    .write_byte(0xFF0F, reset_bit(3, interrupt_flags) as u8);

                self.set_registry_value("PC", 0x0058);
            }

            // joypad
            else if (interrupts & 0b10000) != 0 {
                println!("Handling joypad");

                self.mmu
                    .write_byte(0xFF0F, reset_bit(4, interrupt_flags) as u8);

                self.set_registry_value("PC", 0x0060);
            }
        }

        // todo: on button press resume from stop
        self.regs.write_byte(REG_T, interrupt_cycles_t);
    }

    pub fn execute(&mut self, opcode: u8, cb: bool) {

        if cb == false {
            match opcode {
                0x00 => self.x00(),
                0x01 => self.x01(),
                0x02 => self.x02(),
                0x03 => self.x03(),
                0x04 => self.x04(),
                0x05 => self.x05(),
                0x06 => self.x06(),
                0x07 => self.x07(),
                0x08 => self.x08(),
                0x09 => self.x09(),
                0x0A => self.x0A(),
                0x0B => self.x0B(),
                0x0C => self.x0C(),
                0x0D => self.x0D(),
                0x0E => self.x0E(),
                0x0F => self.x0F(),
                0x10 => self.x10(),
                0x11 => self.x11(),
                0x12 => self.x12(),
                0x13 => self.x13(),
                0x14 => self.x14(),
                0x15 => self.x15(),
                0x16 => self.x16(),
                0x17 => self.x17(),
                0x18 => self.x18(),
                0x19 => self.x19(),
                0x1A => self.x1A(),
                0x1B => self.x1B(),
                0x1C => self.x1C(),
                0x1D => self.x1D(),
                0x1E => self.x1E(),
                0x1F => self.x1F(),
                0x20 => self.x20(),
                0x21 => self.x21(),
                0x22 => self.x22(),
                0x23 => self.x23(),
                0x24 => self.x24(),
                0x25 => self.x25(),
                0x26 => self.x26(),
                0x27 => self.x27(),
                0x28 => self.x28(),
                0x29 => self.x29(),
                0x2A => self.x2A(),
                0x2B => self.x2B(),
                0x2C => self.x2C(),
                0x2D => self.x2D(),
                0x2E => self.x2E(),
                0x2F => self.x2F(),
                0x30 => self.x30(),
                0x31 => self.x31(),
                0x32 => self.x32(),
                0x33 => self.x33(),
                0x34 => self.x34(),
                0x35 => self.x35(),
                0x36 => self.x36(),
                0x37 => self.x37(),
                0x38 => self.x38(),
                0x39 => self.x39(),
                0x3A => self.x3A(),
                0x3B => self.x3B(),
                0x3C => self.x3C(),
                0x3D => self.x3D(),
                0x3E => self.x3E(),
                0x3F => self.x3F(),
                0x40 => self.x40(),
                0x41 => self.x41(),
                0x42 => self.x42(),
                0x43 => self.x43(),
                0x44 => self.x44(),
                0x45 => self.x45(),
                0x46 => self.x46(),
                0x47 => self.x47(),
                0x48 => self.x48(),
                0x49 => self.x49(),
                0x4A => self.x4A(),
                0x4B => self.x4B(),
                0x4C => self.x4C(),
                0x4D => self.x4D(),
                0x4E => self.x4E(),
                0x4F => self.x4F(),
                0x50 => self.x50(),
                0x51 => self.x51(),
                0x52 => self.x52(),
                0x53 => self.x53(),
                0x54 => self.x54(),
                0x55 => self.x55(),
                0x56 => self.x56(),
                0x57 => self.x57(),
                0x58 => self.x58(),
                0x59 => self.x59(),
                0x5A => self.x5A(),
                0x5B => self.x5B(),
                0x5C => self.x5C(),
                0x5D => self.x5D(),
                0x5E => self.x5E(),
                0x5F => self.x5F(),
                0x60 => self.x60(),
                0x61 => self.x61(),
                0x62 => self.x62(),
                0x63 => self.x63(),
                0x64 => self.x64(),
                0x65 => self.x65(),
                0x66 => self.x66(),
                0x67 => self.x67(),
                0x68 => self.x68(),
                0x69 => self.x69(),
                0x6A => self.x6A(),
                0x6B => self.x6B(),
                0x6C => self.x6C(),
                0x6D => self.x6D(),
                0x6E => self.x6E(),
                0x6F => self.x6F(),
                0x70 => self.x70(),
                0x71 => self.x71(),
                0x72 => self.x72(),
                0x73 => self.x73(),
                0x74 => self.x74(),
                0x75 => self.x75(),
                0x76 => self.x76(),
                0x77 => self.x77(),
                0x78 => self.x78(),
                0x79 => self.x79(),
                0x7A => self.x7A(),
                0x7B => self.x7B(),
                0x7C => self.x7C(),
                0x7D => self.x7D(),
                0x7E => self.x7E(),
                0x7F => self.x7F(),
                0x80 => self.x80(),
                0x81 => self.x81(),
                0x82 => self.x82(),
                0x83 => self.x83(),
                0x84 => self.x84(),
                0x85 => self.x85(),
                0x86 => self.x86(),
                0x87 => self.x87(),
                0x88 => self.x88(),
                0x89 => self.x89(),
                0x8A => self.x8A(),
                0x8B => self.x8B(),
                0x8C => self.x8C(),
                0x8D => self.x8D(),
                0x8E => self.x8E(),
                0x8F => self.x8F(),
                0x90 => self.x90(),
                0x91 => self.x91(),
                0x92 => self.x92(),
                0x93 => self.x93(),
                0x94 => self.x94(),
                0x95 => self.x95(),
                0x96 => self.x96(),
                0x97 => self.x97(),
                0x98 => self.x98(),
                0x99 => self.x99(),
                0x9A => self.x9A(),
                0x9B => self.x9B(),
                0x9C => self.x9C(),
                0x9D => self.x9D(),
                0x9E => self.x9E(),
                0x9F => self.x9F(),
                0xA0 => self.xA0(),
                0xA1 => self.xA1(),
                0xA2 => self.xA2(),
                0xA3 => self.xA3(),
                0xA4 => self.xA4(),
                0xA5 => self.xA5(),
                0xA6 => self.xA6(),
                0xA7 => self.xA7(),
                0xA8 => self.xA8(),
                0xA9 => self.xA9(),
                0xAA => self.xAA(),
                0xAB => self.xAB(),
                0xAC => self.xAC(),
                0xAD => self.xAD(),
                0xAE => self.xAE(),
                0xAF => self.xAF(),
                0xB0 => self.xB0(),
                0xB1 => self.xB1(),
                0xB2 => self.xB2(),
                0xB3 => self.xB3(),
                0xB4 => self.xB4(),
                0xB5 => self.xB5(),
                0xB6 => self.xB6(),
                0xB7 => self.xB7(),
                0xB8 => self.xB8(),
                0xB9 => self.xB9(),
                0xBA => self.xBA(),
                0xBB => self.xBB(),
                0xBC => self.xBC(),
                0xBD => self.xBD(),
                0xBE => self.xBE(),
                0xBF => self.xBF(),
                0xC0 => self.xC0(),
                0xC1 => self.xC1(),
                0xC2 => self.xC2(),
                0xC3 => self.xC3(),
                0xC4 => self.xC4(),
                0xC5 => self.xC5(),
                0xC6 => self.xC6(),
                0xC7 => self.xC7(),
                0xC8 => self.xC8(),
                0xC9 => self.xC9(),
                0xCA => self.xCA(),
                0xCB => self.xCB(),
                0xCC => self.xCC(),
                0xCD => self.xCD(),
                0xCE => self.xCE(),
                0xCF => self.xCF(),
                0xD0 => self.xD0(),
                0xD1 => self.xD1(),
                0xD2 => self.xD2(),
                0xD3 => self.xD3(),
                0xD4 => self.xD4(),
                0xD5 => self.xD5(),
                0xD6 => self.xD6(),
                0xD7 => self.xD7(),
                0xD8 => self.xD8(),
                0xD9 => self.xD9(),
                0xDA => self.xDA(),
                0xDB => self.xDB(),
                0xDC => self.xDC(),
                0xDD => self.xDD(),
                0xDE => self.xDE(),
                0xDF => self.xDF(),
                0xE0 => self.xE0(),
                0xE1 => self.xE1(),
                0xE2 => self.xE2(),
                0xE3 => self.xE3(),
                0xE4 => self.xE4(),
                0xE5 => self.xE5(),
                0xE6 => self.xE6(),
                0xE7 => self.xE7(),
                0xE8 => self.xE8(),
                0xE9 => self.xE9(),
                0xEA => self.xEA(),
                0xEB => self.xEB(),
                0xEC => self.xEC(),
                0xED => self.xED(),
                0xEE => self.xEE(),
                0xEF => self.xEF(),
                0xF0 => self.xF0(),
                0xF1 => self.xF1(),
                0xF2 => self.xF2(),
                0xF3 => self.xF3(),
                0xF4 => self.xF4(),
                0xF5 => self.xF5(),
                0xF6 => self.xF6(),
                0xF7 => self.xF7(),
                0xF8 => self.xF8(),
                0xF9 => self.xF9(),
                0xFA => self.xFA(),
                0xFB => self.xFB(),
                0xFC => self.xFC(),
                0xFD => self.xFD(),
                0xFE => self.xFE(),
                0xFF => self.xFF(),
                _ => {}
            }
        } else {
            match opcode {
                0x00 => self.xCB00(),
                0x01 => self.xCB01(),
                0x02 => self.xCB02(),
                0x03 => self.xCB03(),
                0x04 => self.xCB04(),
                0x05 => self.xCB05(),
                0x06 => self.xCB06(),
                0x07 => self.xCB07(),
                0x08 => self.xCB08(),
                0x09 => self.xCB09(),
                0x0A => self.xCB0A(),
                0x0B => self.xCB0B(),
                0x0C => self.xCB0C(),
                0x0D => self.xCB0D(),
                0x0E => self.xCB0E(),
                0x0F => self.xCB0F(),
                0x10 => self.xCB10(),
                0x11 => self.xCB11(),
                0x12 => self.xCB12(),
                0x13 => self.xCB13(),
                0x14 => self.xCB14(),
                0x15 => self.xCB15(),
                0x16 => self.xCB16(),
                0x17 => self.xCB17(),
                0x18 => self.xCB18(),
                0x19 => self.xCB19(),
                0x1A => self.xCB1A(),
                0x1B => self.xCB1B(),
                0x1C => self.xCB1C(),
                0x1D => self.xCB1D(),
                0x1E => self.xCB1E(),
                0x1F => self.xCB1F(),
                0x20 => self.xCB20(),
                0x21 => self.xCB21(),
                0x22 => self.xCB22(),
                0x23 => self.xCB23(),
                0x24 => self.xCB24(),
                0x25 => self.xCB25(),
                0x26 => self.xCB26(),
                0x27 => self.xCB27(),
                0x28 => self.xCB28(),
                0x29 => self.xCB29(),
                0x2A => self.xCB2A(),
                0x2B => self.xCB2B(),
                0x2C => self.xCB2C(),
                0x2D => self.xCB2D(),
                0x2E => self.xCB2E(),
                0x2F => self.xCB2F(),
                0x30 => self.xCB30(),
                0x31 => self.xCB31(),
                0x32 => self.xCB32(),
                0x33 => self.xCB33(),
                0x34 => self.xCB34(),
                0x35 => self.xCB35(),
                0x36 => self.xCB36(),
                0x37 => self.xCB37(),
                0x38 => self.xCB38(),
                0x39 => self.xCB39(),
                0x3A => self.xCB3A(),
                0x3B => self.xCB3B(),
                0x3C => self.xCB3C(),
                0x3D => self.xCB3D(),
                0x3E => self.xCB3E(),
                0x3F => self.xCB3F(),
                0x40 => self.xCB40(),
                0x41 => self.xCB41(),
                0x42 => self.xCB42(),
                0x43 => self.xCB43(),
                0x44 => self.xCB44(),
                0x45 => self.xCB45(),
                0x46 => self.xCB46(),
                0x47 => self.xCB47(),
                0x48 => self.xCB48(),
                0x49 => self.xCB49(),
                0x4A => self.xCB4A(),
                0x4B => self.xCB4B(),
                0x4C => self.xCB4C(),
                0x4D => self.xCB4D(),
                0x4E => self.xCB4E(),
                0x4F => self.xCB4F(),
                0x50 => self.xCB50(),
                0x51 => self.xCB51(),
                0x52 => self.xCB52(),
                0x53 => self.xCB53(),
                0x54 => self.xCB54(),
                0x55 => self.xCB55(),
                0x56 => self.xCB56(),
                0x57 => self.xCB57(),
                0x58 => self.xCB58(),
                0x59 => self.xCB59(),
                0x5A => self.xCB5A(),
                0x5B => self.xCB5B(),
                0x5C => self.xCB5C(),
                0x5D => self.xCB5D(),
                0x5E => self.xCB5E(),
                0x5F => self.xCB5F(),
                0x60 => self.xCB60(),
                0x61 => self.xCB61(),
                0x62 => self.xCB62(),
                0x63 => self.xCB63(),
                0x64 => self.xCB64(),
                0x65 => self.xCB65(),
                0x66 => self.xCB66(),
                0x67 => self.xCB67(),
                0x68 => self.xCB68(),
                0x69 => self.xCB69(),
                0x6A => self.xCB6A(),
                0x6B => self.xCB6B(),
                0x6C => self.xCB6C(),
                0x6D => self.xCB6D(),
                0x6E => self.xCB6E(),
                0x6F => self.xCB6F(),
                0x70 => self.xCB70(),
                0x71 => self.xCB71(),
                0x72 => self.xCB72(),
                0x73 => self.xCB73(),
                0x74 => self.xCB74(),
                0x75 => self.xCB75(),
                0x76 => self.xCB76(),
                0x77 => self.xCB77(),
                0x78 => self.xCB78(),
                0x79 => self.xCB79(),
                0x7A => self.xCB7A(),
                0x7B => self.xCB7B(),
                0x7C => self.xCB7C(),
                0x7D => self.xCB7D(),
                0x7E => self.xCB7E(),
                0x7F => self.xCB7F(),
                0x80 => self.xCB80(),
                0x81 => self.xCB81(),
                0x82 => self.xCB82(),
                0x83 => self.xCB83(),
                0x84 => self.xCB84(),
                0x85 => self.xCB85(),
                0x86 => self.xCB86(),
                0x87 => self.xCB87(),
                0x88 => self.xCB88(),
                0x89 => self.xCB89(),
                0x8A => self.xCB8A(),
                0x8B => self.xCB8B(),
                0x8C => self.xCB8C(),
                0x8D => self.xCB8D(),
                0x8E => self.xCB8E(),
                0x8F => self.xCB8F(),
                0x90 => self.xCB90(),
                0x91 => self.xCB91(),
                0x92 => self.xCB92(),
                0x93 => self.xCB93(),
                0x94 => self.xCB94(),
                0x95 => self.xCB95(),
                0x96 => self.xCB96(),
                0x97 => self.xCB97(),
                0x98 => self.xCB98(),
                0x99 => self.xCB99(),
                0x9A => self.xCB9A(),
                0x9B => self.xCB9B(),
                0x9C => self.xCB9C(),
                0x9D => self.xCB9D(),
                0x9E => self.xCB9E(),
                0x9F => self.xCB9F(),
                0xA0 => self.xCBA0(),
                0xA1 => self.xCBA1(),
                0xA2 => self.xCBA2(),
                0xA3 => self.xCBA3(),
                0xA4 => self.xCBA4(),
                0xA5 => self.xCBA5(),
                0xA6 => self.xCBA6(),
                0xA7 => self.xCBA7(),
                0xA8 => self.xCBA8(),
                0xA9 => self.xCBA9(),
                0xAA => self.xCBAA(),
                0xAB => self.xCBAB(),
                0xAC => self.xCBAC(),
                0xAD => self.xCBAD(),
                0xAE => self.xCBAE(),
                0xAF => self.xCBAF(),
                0xB0 => self.xCBB0(),
                0xB1 => self.xCBB1(),
                0xB2 => self.xCBB2(),
                0xB3 => self.xCBB3(),
                0xB4 => self.xCBB4(),
                0xB5 => self.xCBB5(),
                0xB6 => self.xCBB6(),
                0xB7 => self.xCBB7(),
                0xB8 => self.xCBB8(),
                0xB9 => self.xCBB9(),
                0xBA => self.xCBBA(),
                0xBB => self.xCBBB(),
                0xBC => self.xCBBC(),
                0xBD => self.xCBBD(),
                0xBE => self.xCBBE(),
                0xBF => self.xCBBF(),
                0xC0 => self.xCBC0(),
                0xC1 => self.xCBC1(),
                0xC2 => self.xCBC2(),
                0xC3 => self.xCBC3(),
                0xC4 => self.xCBC4(),
                0xC5 => self.xCBC5(),
                0xC6 => self.xCBC6(),
                0xC7 => self.xCBC7(),
                0xC8 => self.xCBC8(),
                0xC9 => self.xCBC9(),
                0xCA => self.xCBCA(),
                0xCB => self.xCBCB(),
                0xCC => self.xCBCC(),
                0xCD => self.xCBCD(),
                0xCE => self.xCBCE(),
                0xCF => self.xCBCF(),
                0xD0 => self.xCBD0(),
                0xD1 => self.xCBD1(),
                0xD2 => self.xCBD2(),
                0xD3 => self.xCBD3(),
                0xD4 => self.xCBD4(),
                0xD5 => self.xCBD5(),
                0xD6 => self.xCBD6(),
                0xD7 => self.xCBD7(),
                0xD8 => self.xCBD8(),
                0xD9 => self.xCBD9(),
                0xDA => self.xCBDA(),
                0xDB => self.xCBDB(),
                0xDC => self.xCBDC(),
                0xDD => self.xCBDD(),
                0xDE => self.xCBDE(),
                0xDF => self.xCBDF(),
                0xE0 => self.xCBE0(),
                0xE1 => self.xCBE1(),
                0xE2 => self.xCBE2(),
                0xE3 => self.xCBE3(),
                0xE4 => self.xCBE4(),
                0xE5 => self.xCBE5(),
                0xE6 => self.xCBE6(),
                0xE7 => self.xCBE7(),
                0xE8 => self.xCBE8(),
                0xE9 => self.xCBE9(),
                0xEA => self.xCBEA(),
                0xEB => self.xCBEB(),
                0xEC => self.xCBEC(),
                0xED => self.xCBED(),
                0xEE => self.xCBEE(),
                0xEF => self.xCBEF(),
                0xF0 => self.xCBF0(),
                0xF1 => self.xCBF1(),
                0xF2 => self.xCBF2(),
                0xF3 => self.xCBF3(),
                0xF4 => self.xCBF4(),
                0xF5 => self.xCBF5(),
                0xF6 => self.xCBF6(),
                0xF7 => self.xCBF7(),
                0xF8 => self.xCBF8(),
                0xF9 => self.xCBF9(),
                0xFA => self.xCBFA(),
                0xFB => self.xCBFB(),
                0xFC => self.xCBFC(),
                0xFD => self.xCBFD(),
                0xFE => self.xCBFE(),
                0xFF => self.xCBFF(),
                _ => {}
            }
        }
    }

    fn x00(&mut self) {
        self.regs.write_byte(REG_T, 4);
    }

    fn x01(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("BC", op1, false);

        self.regs.write_byte(REG_T, 12);
    }

    fn x02(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(BC)", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x03(&mut self) {
        let op1 = self.get_operand_value("BC");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("BC", result, false);

        self.regs.write_byte(REG_T, 8);
    }

    fn x04(&mut self) {
        let op1 = self.get_operand_value("B");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x05(&mut self) {
        let op1 = self.get_operand_value("B");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x06(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("B", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x07(&mut self) {
        let op1 = self.get_operand_value("A");

        let new_carry = (op1 & 0x80) != 0;
        let result = ((op1 as u8) << 1 | u8::from(new_carry)) as u16;

        self.store_result("A", result, true);

        self.regs.set_flags(false, false, false, new_carry);

        self.regs.write_byte(REG_T, 4);
    }

    fn x08(&mut self) {
        let op1 = self.get_operand_value("SP");
        self.store_result("(a16)", op1, false);

        self.regs.write_byte(REG_T, 20);
    }

    fn x09(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("BC");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);

        self.regs.set_flags(old_z, false, h, c);

        self.regs.write_byte(REG_T, 8);
    }

    fn x0A(&mut self) {
        let op1 = self.get_operand_value("(BC)");
        self.store_result("A", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x0B(&mut self) {
        let op1 = self.get_operand_value("BC");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("BC", result, false);

        self.regs.write_byte(REG_T, 8);
    }

    fn x0C(&mut self) {
        let op1 = self.get_operand_value("C");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x0D(&mut self) {
        let op1 = self.get_operand_value("C");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x0E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("C", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x0F(&mut self) {
        let op1 = self.get_operand_value("A");

        let new_carry = (op1 & 1) != 0;
        let result = ((op1 as u8) >> 1 | (u8::from(new_carry) << 7)) as u16;

        self.store_result("A", result, true);

        self.regs.set_flags(false, false, false, new_carry);

        self.regs.write_byte(REG_T, 4);
    }

    fn x10(&mut self) {
        self.stopped = true;

        self.regs.write_byte(REG_T, 4);
    }

    fn x11(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("DE", op1, false);

        self.regs.write_byte(REG_T, 12);
    }

    fn x12(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(DE)", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x13(&mut self) {
        let op1 = self.get_operand_value("DE");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("DE", result, false);

        self.regs.write_byte(REG_T, 8);
    }

    fn x14(&mut self) {
        let op1 = self.get_operand_value("D");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x15(&mut self) {
        let op1 = self.get_operand_value("D");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);

        self.regs.write_byte(REG_T, 4);
    }

    fn x16(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("D", op1, true);

        self.regs.write_byte(REG_T, 8);
    }

    fn x17(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags(false, false, false, new_carry);

        self.regs.write_byte(REG_T, 4);
    }

    fn x18(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.store_result("PC", result, false);

        self.regs.write_byte(REG_T, 12);
    }

    fn x19(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("DE");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);

        self.regs.set_flags(old_z, false, h, c);
    }

    fn x1A(&mut self) {
        let op1 = self.get_operand_value("(DE)");
        self.store_result("A", op1, true);
    }

    fn x1B(&mut self) {
        let op1 = self.get_operand_value("DE");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("DE", result, false);
    }

    fn x1C(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c)
    }

    fn x1D(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn x1E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("E", op1, true);
    }

    fn x1F(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags(false, false, false, new_carry)
    }

    fn x20(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.store_result("PC", result, false);
    }

    fn x21(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("HL", op1, false);
    }

    fn x22(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_add(1), false);
    }

    fn x23(&mut self) {
        let op1 = self.get_operand_value("HL");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("HL", result, false);
    }

    fn x24(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c)
    }

    fn x25(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn x26(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("H", op1, true);
    }

    fn x27(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, prev_n, prev_h, prev_c) = self.regs.get_flags();

        let mut new_carry = prev_c;

        let mut adjust = 0;

        if prev_h {
            adjust |= 0x06;
        }

        if prev_c {
            adjust |= 0x60;
            new_carry = true;
        }

        let result = if prev_n {
            op1.wrapping_sub(adjust)
        } else {
            if op1 & 0x0F > 0x09 {
                adjust |= 0x06;
            }

            if op1 > 0x99 {
                adjust |= 0x60;
                new_carry = true;
            }

            op1.wrapping_add(adjust)
        };

        self.store_result("A", result, true);
        self.regs.set_flags((result as u8) == 0, prev_n, false, new_carry);
    }

    fn x28(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("Z");

        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.store_result("PC", result, false);
    }

    fn x29(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("HL");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);

        self.regs.set_flags(old_z, false, h, c);
    }

    fn x2A(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_add(1), false);
    }

    fn x2B(&mut self) {
        let op1 = self.get_operand_value("HL");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("HL", result, false);
    }

    fn x2C(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c)
    }

    fn x2D(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn x2E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("L", op1, true);
    }

    fn x2F(&mut self) {
        let op1 = self.get_operand_value("A");
        let (z, _, _, c) = self.regs.get_flags();

        self.store_result("A", !op1, true);
        self.regs.set_flags(z, true, true, c)
    }

    fn x30(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.store_result("PC", result, false);
    }

    fn x31(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("SP", op1, false);
    }

    fn x32(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_sub(1), false);
    }

    fn x33(&mut self) {
        let op1 = self.get_operand_value("SP");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("SP", result, false);
    }

    fn x34(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c)
    }

    fn x35(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn x36(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("(HL)", op1, true);
    }

    fn x37(&mut self) {
        let (z, _, _, _) = self.regs.get_flags();

        self.regs.set_flags(z, false, false, true);
    }

    fn x38(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.store_result("PC", result, false);
    }

    fn x39(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("SP");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);

        self.regs.set_flags(old_z, false, h, c);
    }

    fn x3A(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_sub(1), false);
    }

    fn x3B(&mut self) {
        let op1 = self.get_operand_value("SP");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("SP", result, false);
    }

    fn x3C(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c)
    }

    fn x3D(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn x3E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("A", op1, true);
    }

    fn x3F(&mut self) {
        let (z, _, _, c) = self.regs.get_flags();

        self.regs.set_flags(z, false, false, !c);
    }

    fn x40(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("B", op1, true);
    }

    fn x41(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("B", op1, true);
    }

    fn x42(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("B", op1, true);
    }

    fn x43(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("B", op1, true);
    }

    fn x44(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("B", op1, true);
    }

    fn x45(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("B", op1, true);
    }

    fn x46(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("B", op1, true);
    }

    fn x47(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("B", op1, true);
    }

    fn x48(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("C", op1, true);
    }

    fn x49(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("C", op1, true);
    }

    fn x4A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("C", op1, true);
    }

    fn x4B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("C", op1, true);
    }

    fn x4C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("C", op1, true);
    }

    fn x4D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("C", op1, true);
    }

    fn x4E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("C", op1, true);
    }

    fn x4F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("C", op1, true);
    }

    fn x50(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("D", op1, true);
    }

    fn x51(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("D", op1, true);
    }

    fn x52(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("D", op1, true);
    }

    fn x53(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("D", op1, true);
    }

    fn x54(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("D", op1, true);
    }

    fn x55(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("D", op1, true);
    }

    fn x56(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("D", op1, true);
    }

    fn x57(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("D", op1, true);
    }

    fn x58(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("E", op1, true);
    }

    fn x59(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("E", op1, true);
    }

    fn x5A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("E", op1, true);
    }

    fn x5B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("E", op1, true);
    }

    fn x5C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("E", op1, true);
    }

    fn x5D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("E", op1, true);
    }

    fn x5E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("E", op1, true);
    }

    fn x5F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("E", op1, true);
    }

    fn x60(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("H", op1, true);
    }

    fn x61(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("H", op1, true);
    }

    fn x62(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("H", op1, true);
    }

    fn x63(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("H", op1, true);
    }

    fn x64(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("H", op1, true);
    }

    fn x65(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("H", op1, true);
    }

    fn x66(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("H", op1, true);
    }

    fn x67(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("H", op1, true);
    }

    fn x68(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("L", op1, true);
    }

    fn x69(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("L", op1, true);
    }

    fn x6A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("L", op1, true);
    }

    fn x6B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("L", op1, true);
    }

    fn x6C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("L", op1, true);
    }

    fn x6D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("L", op1, true);
    }

    fn x6E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("L", op1, true);
    }

    fn x6F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("L", op1, true);
    }

    fn x70(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("(HL)", op1, true);
    }

    fn x71(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("(HL)", op1, true);
    }

    fn x72(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("(HL)", op1, true);
    }

    fn x73(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("(HL)", op1, true);
    }

    fn x74(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("(HL)", op1, true);
    }

    fn x75(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("(HL)", op1, true);
    }

    fn x76(&mut self) {
        // todo: implement halt bug
        self.halted = true;
    }

    fn x77(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);
    }

    fn x78(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("A", op1, true);
    }

    fn x79(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("A", op1, true);
    }

    fn x7A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("A", op1, true);
    }

    fn x7B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("A", op1, true);
    }

    fn x7C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("A", op1, true);
    }

    fn x7D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("A", op1, true);
    }

    fn x7E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);
    }

    fn x7F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("A", op1, true);
    }

    fn x80(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x81(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x82(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x83(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x84(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x85(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x86(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x87(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x88(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x89(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8A(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8B(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8C(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8D(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8E(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x8F(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn x90(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x91(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x92(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x93(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x94(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x95(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x96(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x97(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x98(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x99(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9A(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9B(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9C(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9D(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9E(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn x9F(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn xA0(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA1(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA2(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA3(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA4(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA5(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA7(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xA8(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xA9(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAA(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAB(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAC(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAD(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xAF(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB0(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB1(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB2(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB3(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB4(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB5(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB7(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xB8(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xB9(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBA(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBB(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBC(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBD(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xBF(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xC0(&mut self) {
        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let op1 = self.pop();
        self.store_result("PC", op1, false);
    }

    fn xC1(&mut self) {
        let op1 = self.pop();
        self.store_result("BC", op1, false);
    }

    fn xC2(&mut self) {
        let op1 = self.get_operand_value("a16");
        let cond = self.get_operand_value("NZ");

        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        self.store_result("PC", op1, false);
    }

    fn xC3(&mut self) {
        let op1 = self.get_operand_value("a16");
        self.store_result("PC", op1, false);
    }

    fn xC4(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let value = self.get_registry_value("PC");
        self.push(value);

        self.store_result("PC", op1, false);
    }

    fn xC5(&mut self) {
        let op1 = self.get_operand_value("BC");
        self.push(op1);
    }

    fn xC6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn xC7(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x00, false);
    }

    fn xC8(&mut self) {
        let cond = self.get_operand_value("Z");

        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let op1 = self.pop();
        self.store_result("PC", op1, false);
    }

    fn xC9(&mut self) {
        let op1 = self.pop();
        self.store_result("PC", op1, false);
    }

    fn xCA(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("Z");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        self.store_result("PC", op1, false);
    }

    fn xCB(&mut self) { panic!("wtf?") }

    fn xCC(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("Z");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let value = self.get_registry_value("PC");
        self.push(value);

        self.store_result("PC", op1, false);
    }

    fn xCD(&mut self) {
        let op1 = self.get_operand_value("a16");

        let value = self.get_registry_value("PC");
        self.push(value);

        self.store_result("PC", op1, false);
    }

    fn xCE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
    }

    fn xCF(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x08, false);
    }

    fn xD0(&mut self) {
        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let op1 = self.pop();
        self.store_result("PC", op1, false);
    }

    fn xD1(&mut self) {
        let op1 = self.pop();
        self.store_result("DE", op1, false);
    }

    fn xD2(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        self.store_result("PC", op1, false);
    }

    fn xD3(&mut self) {}

    fn xD4(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let value = self.get_registry_value("PC");
        self.push(value);

        self.store_result("PC", op1, false);
    }

    fn xD5(&mut self) {
        let op1 = self.get_operand_value("DE");
        self.push(op1);
    }

    fn xD6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn xD7(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x10, false);
    }

    fn xD8(&mut self) {
        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let op1 = self.pop();
        self.store_result("PC", op1, false);
    }

    fn xD9(&mut self) {
        let op1 = self.pop();
        self.store_result("PC", op1, false);

        self.interrupt_master_enable = true;
    }

    fn xDA(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        self.store_result("PC", op1, false);
    }

    fn xDB(&mut self) {}

    fn xDC(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_M, 1);
            return;
        }

        let value = self.get_registry_value("PC");
        self.push(value);

        self.store_result("PC", op1, false);
    }

    fn xDD(&mut self) {}

    fn xDE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 == true { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
    }

    fn xDF(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x18, false);
    }

    fn xE0(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(a8)", op1, true);
    }

    fn xE1(&mut self) {
        let op1 = self.pop();
        self.store_result("HL", op1, false);
    }

    fn xE2(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(C)", op1, true);
    }

    fn xE3(&mut self) {}

    fn xE4(&mut self) {}

    fn xE5(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.push(op1);
    }

    fn xE6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
    }

    fn xE7(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x20, false);
    }

    fn xE8(&mut self) {
        let op1 = self.get_operand_value("SP");
        let op2 = self.get_operand_value("r8");

        let (result, c, h) = add_word_with_signed(op1, op2, 0);

        self.store_result("SP", result, false);

        self.regs.set_flags(false, false, h, c);
    }

    fn xE9(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.store_result("PC", op1, false);
    }

    fn xEA(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(a16)", op1, true);
    }

    fn xEB(&mut self) {

    }

    fn xEC(&mut self) {

    }

    fn xED(&mut self) {

    }

    fn xEE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xEF(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x28, false);
    }

    fn xF0(&mut self) {
        let op1 = self.get_operand_value("(a8)");
        self.store_result("A", op1, true);
    }

    fn xF1(&mut self) {
        let op1 = self.pop();
        self.store_result("AF", op1, false);
    }

    fn xF2(&mut self) {
        let op1 = self.get_operand_value("(C)");
        self.store_result("A", op1, true);
    }

    fn xF3(&mut self) {
        self.interrupt_master_enable = false
    }

    fn xF4(&mut self) {

    }

    fn xF5(&mut self) {
        let op1 = self.get_operand_value("AF");
        self.push(op1);
    }

    fn xF6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false);
    }

    fn xF7(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x30, false);
    }

    fn xF8(&mut self) {
        let op1 = self.get_operand_value("SP");
        let op2 = self.get_operand_value("r8");

        let (result, c, h) = add_word_with_signed(op1, op2, 0);

        self.store_result("HL", result, false);

        self.regs.set_flags(false, false, h, c);
    }

    fn xF9(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.store_result("SP", op1, false);
    }

    fn xFA(&mut self) {
        let op1 = self.get_operand_value("(a16)");
        self.store_result("A", op1, true);
    }

    fn xFB(&mut self) {
        self.schedule_interrupt_enable = true;
    }

    fn xFC(&mut self) {

    }

    fn xFD(&mut self) {

    }

    fn xFE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
    }

    fn xFF(&mut self) {
        let value = self.get_registry_value("PC");
        self.push(value);
        self.store_result("PC", 0x38, false);
    }

    fn xCB00(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB01(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB02(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB03(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB04(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB05(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB06(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB07(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = (op1 << 1) | (op1 >> 7);
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB08(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB09(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0A(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0B(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0C(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0D(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0E(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB0F(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = (op1 >> 1) | (op1 << 7);
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB10(&mut self) {
        let op1 = self.get_operand_value("B");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB11(&mut self) {
        let op1 = self.get_operand_value("C");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB12(&mut self) {
        let op1 = self.get_operand_value("D");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB13(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB14(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB15(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB16(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB17(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) << 1 | u8::from(prev_c)) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB18(&mut self) {
        let op1 = self.get_operand_value("B");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB19(&mut self) {
        let op1 = self.get_operand_value("C");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1A(&mut self) {
        let op1 = self.get_operand_value("D");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1B(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1C(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1D(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1E(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB1F(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) =  self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }


    fn xCB20(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB21(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB22(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB23(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB24(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB25(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB26(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB27(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = ((op1 as u8) << 1) as u16;
        let new_carry = (op1 & 0x80) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB28(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB29(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2A(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2B(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2C(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2D(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2E(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB2F(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = (op1 >> 1) | (op1 & 0x80);
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry)
    }

    fn xCB30(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = swap_nibbles(op1 as u8);

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB31(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = swap_nibbles(op1 as u8);

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB32(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = swap_nibbles(op1 as u8);

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB33(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = swap_nibbles(op1 as u8);

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB34(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = swap_nibbles(op1 as u8);

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB35(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = swap_nibbles(op1 as u8);

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB36(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = swap_nibbles(op1 as u8);

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB37(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = swap_nibbles(op1 as u8);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, false)
    }

    fn xCB38(&mut self) {
        let op1 = self.get_operand_value("B");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("B", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB39(&mut self) {
        let op1 = self.get_operand_value("C");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("C", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3A(&mut self) {
        let op1 = self.get_operand_value("D");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("D", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3B(&mut self) {
        let op1 = self.get_operand_value("E");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3C(&mut self) {
        let op1 = self.get_operand_value("H");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3D(&mut self) {
        let op1 = self.get_operand_value("L");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3E(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB3F(&mut self) {
        let op1 = self.get_operand_value("A");

        let result = op1 >> 1;
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, false, new_carry);
    }

    fn xCB40(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB41(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB42(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB43(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB44(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB45(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB46(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB47(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(0, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB48(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB49(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4A(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4B(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4C(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4D(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB4F(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(1, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB50(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB51(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB52(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB53(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB54(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB55(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB56(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB57(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(2, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB58(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB59(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5A(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5B(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5C(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5D(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB5F(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(3, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB60(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB61(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB62(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB63(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB64(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB65(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB66(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB67(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(4, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB68(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB69(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6A(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6B(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6C(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6D(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB6F(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(5, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB70(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB71(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB72(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB73(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB74(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB75(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB76(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB77(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(6, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB78(&mut self) {
        let op2 = self.get_operand_value("B");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB79(&mut self) {
        let op2 = self.get_operand_value("C");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7A(&mut self) {
        let op2 = self.get_operand_value("D");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7B(&mut self) {
        let op2 = self.get_operand_value("E");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7C(&mut self) {
        let op2 = self.get_operand_value("H");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7D(&mut self) {
        let op2 = self.get_operand_value("L");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB7F(&mut self) {
        let op2 = self.get_operand_value("A");
        let (_, _, _, old_c) = self.regs.get_flags();

        let result = is_bit_set(7, op2) as u16;

        self.regs.set_flags((result as u8) == 0, false, true, old_c);
    }

    fn xCB80(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(0, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCB81(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(0, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCB82(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(0, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCB83(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(0, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCB84(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(0, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCB85(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(0, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCB86(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(0, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCB87(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(0, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCB88(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(1, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCB89(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(1, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCB8A(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(1, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCB8B(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(1, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCB8C(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(1, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCB8D(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(1, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCB8E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(1, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCB8F(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(1, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCB90(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(2, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCB91(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(2, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCB92(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(2, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCB93(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(2, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCB94(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(2, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCB95(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(2, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCB96(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(2, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCB97(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(2, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCB98(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(3, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCB99(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(3, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCB9A(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(3, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCB9B(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(3, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCB9C(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(3, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCB9D(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(3, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCB9E(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(3, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCB9F(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(3, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBA0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(4, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBA1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(4, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBA2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(4, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBA3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(4, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBA4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(4, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBA5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(4, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBA6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(4, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBA7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(4, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBA8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(5, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBA9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(5, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBAA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(5, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBAB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(5, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBAC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(5, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBAD(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(5, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBAE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(5, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBAF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(5, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBB0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(6, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBB1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(6, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBB2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(6, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBB3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(6, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBB4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(6, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBB5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(6, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBB6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(6, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBB7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(6, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBB8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = reset_bit(7, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBB9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = reset_bit(7, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBBA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = reset_bit(7, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBBB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = reset_bit(7, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBBC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = reset_bit(7, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBBD(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = reset_bit(7, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBBE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = reset_bit(7, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBBF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = reset_bit(7, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBC0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(0, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBC1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(0, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBC2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(0, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBC3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(0, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBC4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(0, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBC5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(0, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBC6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(0, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBC7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(0, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBC8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(1, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBC9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(1, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBCA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(1, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBCB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(1, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBCC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(1, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBCD(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(1, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBCE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(1, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBCF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(1, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBD0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(2, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBD1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(2, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBD2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(2, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBD3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(2, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBD4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(2, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBD5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(2, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBD6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(2, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBD7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(2, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBD8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(3, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBD9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(3, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBDA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(3, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBDB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(3, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBDC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(3, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBDD(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(3, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBDE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(3, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBDF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(3, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBE0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(4, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBE1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(4, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBE2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(4, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBE3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(4, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBE4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(4, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBE5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(4, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBE6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(4, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBE7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(4, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBE8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(5, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBE9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(5, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBEA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(5, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBEB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(5, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBEC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(5, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBED(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(5, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBEE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(5, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBEF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(5, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBF0(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(6, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBF1(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(6, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBF2(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(6, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBF3(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(6, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBF4(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(6, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBF5(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(6, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBF6(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(6, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBF7(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(6, op2 as u8);
        self.store_result("A", result, true);
    }

    fn xCBF8(&mut self) {
        let op2 = self.get_operand_value("B");
        let result = set_bit(7, op2 as u8);
        self.store_result("B", result, true);
    }

    fn xCBF9(&mut self) {
        let op2 = self.get_operand_value("C");
        let result = set_bit(7, op2 as u8);
        self.store_result("C", result, true);
    }

    fn xCBFA(&mut self) {
        let op2 = self.get_operand_value("D");
        let result = set_bit(7, op2 as u8);
        self.store_result("D", result, true);
    }

    fn xCBFB(&mut self) {
        let op2 = self.get_operand_value("E");
        let result = set_bit(7, op2 as u8);
        self.store_result("E", result, true);
    }

    fn xCBFC(&mut self) {
        let op2 = self.get_operand_value("H");
        let result = set_bit(7, op2 as u8);
        self.store_result("H", result, true);
    }

    fn xCBFD(&mut self) {
        let op2 = self.get_operand_value("L");
        let result = set_bit(7, op2 as u8);
        self.store_result("L", result, true);
    }

    fn xCBFE(&mut self) {
        let op2 = self.get_operand_value("(HL)");
        let result = set_bit(7, op2 as u8);
        self.store_result("(HL)", result, true);
    }

    fn xCBFF(&mut self) {
        let op2 = self.get_operand_value("A");
        let result = set_bit(7, op2 as u8);
        self.store_result("A", result, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyMMU {
        values: [u8; 65536],
    }

    impl DummyMMU {
        fn new() -> DummyMMU {
            DummyMMU { values: [0; 65536] }
        }
        fn with(values: [u8; 65536]) -> DummyMMU {
            DummyMMU { values }
        }
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
        assert_eq!(regs.read_word(REG_PC), 0x100);
        assert_eq!(regs.read_word(REG_SP), 0xFFFE);
        assert_eq!(regs.read_byte(REG_M), 0);
        assert_eq!(regs.read_byte(REG_T), 0);
    }

    #[test]
    fn get_flags() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.regs.set_flags(true, false, true, false);
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

        assert_eq!(cpu.get_registry_value("PC"), 500);
    }

    #[test]
    fn test_push() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.push(0xF000);
        cpu.push(0x0F01);
        cpu.push(0x1110);

        assert_eq!(cpu.pop(), 0x1110);
        assert_eq!(cpu.pop(), 0x0F01);
        assert_eq!(cpu.pop(), 0xF000);
    }

    #[test]
    fn test_pop_af() {
        let mut cpu = CPU::new(DummyMMU::new());

        // push to SP
        cpu.push(0xEEFF);

        // set next instrucion to POP AF
        cpu.set_registry_value("PC", 500);
        cpu.mmu.values[500] = 0xF1;

        // execute it
        cpu.step();

        // lower nibble of F must be untouched
        assert_eq!(cpu.get_registry_value("F"), 0xF0)
    }
}
