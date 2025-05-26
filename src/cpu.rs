#![allow(non_snake_case)]

use crate::mem::Memory;
use crate::utils::add_bytes;
use crate::utils::add_word_with_signed;
use crate::utils::add_words;
use crate::utils::parse_hex;
use crate::utils::reset_bit;
use crate::utils::set_bit;
use crate::utils::sub_bytes;
use crate::utils::swap_nibbles;

pub const CPU_FREQ: usize = 4194304; // cpu frequency, in hz

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
    m: u32,
    t: u32,
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

fn cb_rlc(val: u16) -> (u16, bool) {
    ((val << 1) | (val >> 7), (val & 0x80) != 0)
}
fn cb_rrc(val: u16) -> (u16, bool) {
    ((val >> 1) | (val << 7), (val & 1) != 0)
}
fn cb_rl(val: u16, carry: bool) -> (u16, bool) {
    (((val as u8) << 1 | u8::from(carry)) as u16, (val & 0x80) != 0)
}
fn cb_rr(val: u16, carry: bool) -> (u16, bool) {
    (((val as u8) >> 1 | (u8::from(carry) << 7)) as u16, (val & 1) != 0)
}
fn cb_sla(val: u16) -> (u16, bool) {
    (((val as u8) << 1) as u16, (val & 0x80) != 0)
}
fn cb_sra(val: u16) -> (u16, bool) {
    ((val >> 1) | (val & 0x80), (val & 1) != 0)
}
fn cb_swap(val: u16) -> (u16, bool) {
    (swap_nibbles(val as u8), false)
}
fn cb_srl(val: u16) -> (u16, bool) {
    (val >> 1, (val & 1) != 0)
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
    fn write_word(&mut self, addr: u16, word: u16) {
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
    halted: bool,
    halt_bug: bool, // HALT with IME=0 + pending interrupt: next opcode byte is read twice
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
            halt_bug: false,
        };
        cpu.reset();
        cpu
    }

    // initalize
    fn reset(&mut self) {
        self.set_registry_value("AF", 0x01B0);
        self.set_registry_value("BC", 0x0013);
        self.set_registry_value("DE", 0x00D8);
        self.set_registry_value("HL", 0x014D);
        self.set_registry_value("SP", 0xFFFE);
        self.set_registry_value("PC", 0x100);
        self.interrupt_master_enable = true;
    }

    fn tick_m(&mut self) {
        self.mmu.tick_t();
        self.mmu.tick_t();
        self.mmu.tick_t();
        self.mmu.tick_t();
    }

    // fetches the next byte from the ram, advancing one M-cycle
    fn fetch_next_byte(&mut self) -> u8 {
        let pc = self.regs.read_word(REG_PC);
        let byte = self.mmu.read_byte(pc);
        self.regs.write_word(REG_PC, pc.wrapping_add(1));
        self.tick_m();
        byte
    }

    // fetches the next word from the ram (two M-cycles)
    fn fetch_next_word(&mut self) -> u16 {
        let low = self.fetch_next_byte() as u16;
        let high = self.fetch_next_byte() as u16;
        low | (high << 8)
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
        let addr: u16 = match into {
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
            self.mmu.write_byte(addr, value as u8);
            self.tick_m();
        } else {
            self.mmu.write_byte(addr, (value & 0xFF) as u8);
            self.tick_m();
            self.mmu
                .write_byte(addr.wrapping_add(1), ((value >> 8) & 0xFF) as u8);
            self.tick_m();
        }
    }

    pub fn get_operand_value(&mut self, operand: &str) -> u16 {
        match operand {
            "(BC)" | "(DE)" | "(HL)" | "(PC)" | "(SP)" => {
                let reg = operand[1..operand.len() - 1].as_ref();
                let addr = self.get_registry_value(reg);
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            "BC" | "DE" | "HL" | "PC" | "SP" | "AF" | "A" | "B" | "C" | "D" | "E" | "H" | "L" => {
                self.get_registry_value(operand)
            }
            "(a8)" => {
                let addr = 0xFF00 + u16::from(self.fetch_next_byte());
                let val = u16::from(self.mmu.read_byte(addr));
                self.tick_m();
                val
            }
            "(C)" => {
                let addr = 0xFF00 + self.get_registry_value("C");
                let val = u16::from(self.mmu.read_byte(addr));
                self.tick_m();
                val
            }
            "(a16)" => {
                let addr = self.fetch_next_word();
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
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
        self.mmu
            .write_byte(sp.wrapping_sub(1), ((value >> 8) & 0xFF) as u8);
        self.tick_m();
        self.mmu
            .write_byte(sp.wrapping_sub(2), (value & 0xFF) as u8);
        self.tick_m();
        self.set_registry_value("SP", sp.wrapping_sub(2));
    }

    pub fn pop(&mut self) -> u16 {
        let sp = self.get_registry_value("SP");
        let low = self.mmu.read_byte(sp) as u16;
        self.tick_m();
        let high = self.mmu.read_byte(sp.wrapping_add(1)) as u16;
        self.tick_m();
        self.set_registry_value("SP", sp.wrapping_add(2));
        low | (high << 8)
    }

    // executes the next instruction
    // returns instruction executed and cycles taken
    pub fn step(&mut self) -> (u16, u8) {
        let mut instr: u16 = 0;

        if !self.halted {
            let mut prefixed = false;
            // Snapshot GPU mode/row before the opcode fetch ticks the GPU.
            self.mmu.before_fetch();
            let mut byte = self.read_byte();

            if self.halt_bug {
                // Rewind PC: the byte after HALT gets used as both opcode and first operand
                let pc = self.regs.read_word(REG_PC);
                self.regs.write_word(REG_PC, pc.wrapping_sub(1));
                self.halt_bug = false;
            }

            if byte == 0xcb {
                byte = self.read_byte();
                instr = 0xcb00 | (byte as u16);

                prefixed = true;
            } else {
                instr = byte as u16;
            }
            // Capture register value pre-execute: INC/DEC changes the register,
            // so we need the bus address as it was at M1, not after the instruction.
            let oam_rr: Option<u16> = if !prefixed {
                match byte {
                    0x03 | 0x0B => Some(self.get_registry_value("BC")),
                    0x13 | 0x1B => Some(self.get_registry_value("DE")),
                    0x23 | 0x2B | 0x2A | 0x3A => Some(self.get_registry_value("HL")),
                    0x33 | 0x3B => Some(self.get_registry_value("SP")),
                    0xC1 | 0xD1 | 0xE1 | 0xF1 | 0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                        Some(self.get_registry_value("SP"))
                    }
                    _ => None,
                }
            } else {
                None
            };

            self.enable_ime_if_scheduled();
            self.execute(byte, prefixed);

            if let Some(rr) = oam_rr {
                self.mmu.handle_oam_corruption(byte, rr);
            }
        } else {
            self.tick_m();
            self.regs.write_byte(REG_T, 4);
        }

        (instr, self.regs.read_byte(REG_T))
    }

    pub fn enable_ime_if_scheduled(&mut self) {
        if self.schedule_interrupt_enable {
            self.interrupt_master_enable = true;
            self.schedule_interrupt_enable = false;
        }
    }

    // return IE & IF
    fn interrupts_to_handle(&mut self) -> u8 {
        let interrupt_enable = self.mmu.read_byte(0xFFFF);
        let interrupt_flags = self.mmu.read_byte(0xFF0F);
        interrupt_enable & interrupt_flags
    }

    pub fn handle_interrupts(&mut self) -> u8 {
        let interrupts = self.interrupts_to_handle();

        // wake up cpu if there is an interrupt, even if ime = 0
        if interrupts != 0 && self.halted {
            self.halted = false;
        }

        // if we have to handle an interrupt
        if self.interrupt_master_enable && interrupts != 0 {
            // only one interrupt handling at a time
            self.interrupt_master_enable = false;

            // M1, M2: internal cycles (pipeline flush)
            self.tick_m();
            self.tick_m();

            // M3: push PC high byte
            let pc = self.get_registry_value("PC");
            let sp = self.get_registry_value("SP");
            self.mmu.write_byte(sp.wrapping_sub(1), ((pc >> 8) & 0xFF) as u8);
            self.tick_m();

            // Hardware re-reads IE & IF after M3 to determine the vector.
            // If IE changed during M3 (e.g. an ISR wrote to it), the new value wins.
            // If pending becomes 0, dispatch is "cancelled" and PC lands at $0000.
            let pending = self.interrupts_to_handle();

            // M4: push PC low byte
            self.mmu.write_byte(sp.wrapping_sub(2), (pc & 0xFF) as u8);
            self.tick_m();
            self.set_registry_value("SP", sp.wrapping_sub(2));

            // M5: load vector
            self.tick_m();

            let interrupt_flags = self.mmu.read_byte(0xFF0F);

            if (pending & 0x01) != 0 {
                self.mmu.write_byte(0xFF0F, reset_bit(0, interrupt_flags) as u8);
                self.set_registry_value("PC", 0x0040);
            } else if (pending & 0x02) != 0 {
                self.mmu.write_byte(0xFF0F, reset_bit(1, interrupt_flags) as u8);
                self.set_registry_value("PC", 0x0048);
            } else if (pending & 0x04) != 0 {
                self.mmu.write_byte(0xFF0F, reset_bit(2, interrupt_flags) as u8);
                self.set_registry_value("PC", 0x0050);
            } else if (pending & 0x08) != 0 {
                self.mmu.write_byte(0xFF0F, reset_bit(3, interrupt_flags) as u8);
                self.set_registry_value("PC", 0x0058);
            } else if (pending & 0x10) != 0 {
                self.mmu.write_byte(0xFF0F, reset_bit(4, interrupt_flags) as u8);
                self.set_registry_value("PC", 0x0060);
            } else {
                // All bits cleared before vector load — PC becomes $0000
                self.set_registry_value("PC", 0x0000);
            }

            return 20;
        }

        0
    }

    pub fn execute(&mut self, opcode: u8, cb: bool) {
        if !cb {
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
                0x20 | 0x28 | 0x30 | 0x38 => self.cond_jr(opcode),
                0x21 => self.x21(),
                0x22 => self.x22(),
                0x23 => self.x23(),
                0x24 => self.x24(),
                0x25 => self.x25(),
                0x26 => self.x26(),
                0x27 => self.x27(),
                0x29 => self.x29(),
                0x2A => self.x2A(),
                0x2B => self.x2B(),
                0x2C => self.x2C(),
                0x2D => self.x2D(),
                0x2E => self.x2E(),
                0x2F => self.x2F(),
                0x31 => self.x31(),
                0x32 => self.x32(),
                0x33 => self.x33(),
                0x34 => self.x34(),
                0x35 => self.x35(),
                0x36 => self.x36(),
                0x37 => self.x37(),
                0x39 => self.x39(),
                0x3A => self.x3A(),
                0x3B => self.x3B(),
                0x3C => self.x3C(),
                0x3D => self.x3D(),
                0x3E => self.x3E(),
                0x3F => self.x3F(),
                // LD r, r': dst = bits 5-3, src = bits 2-0
                0x40..=0x75 | 0x77..=0x7F => {
                    let src = Self::cb_reg_name(opcode & 0x07);
                    let dst = Self::cb_reg_name((opcode >> 3) & 0x07);
                    let val = self.get_operand_value(src);
                    self.store_result(dst, val, true);
                    let is_hl = (opcode & 0x07) == 6 || ((opcode >> 3) & 0x07) == 6;
                    self.regs.write_byte(REG_T, if is_hl { 8 } else { 4 });
                }
                0x76 => self.x76(),
                // ALU A, r: op = bits 5-3, src = bits 2-0
                0x80..=0xBF => self.alu_a_r(opcode),
                // RET cc
                0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                    self.tick_m();
                    if !self.cond_met(opcode) {
                        self.regs.write_byte(REG_T, 8);
                        return;
                    }
                    let addr = self.pop();
                    self.tick_m();
                    self.store_result("PC", addr, false);
                    self.regs.write_byte(REG_T, 20);
                }
                0xC1 => self.xC1(),
                // JP cc, a16
                0xC2 | 0xCA | 0xD2 | 0xDA => {
                    let addr = self.get_operand_value("a16");
                    if !self.cond_met(opcode) {
                        self.regs.write_byte(REG_T, 12);
                        return;
                    }
                    self.tick_m();
                    self.store_result("PC", addr, false);
                    self.regs.write_byte(REG_T, 16);
                }
                0xC3 => self.xC3(),
                // CALL cc, a16
                0xC4 | 0xCC | 0xD4 | 0xDC => {
                    let addr = self.get_operand_value("a16");
                    if !self.cond_met(opcode) {
                        self.regs.write_byte(REG_T, 12);
                        return;
                    }
                    let pc = self.get_registry_value("PC");
                    self.tick_m();
                    self.push(pc);
                    self.store_result("PC", addr, false);
                    self.regs.write_byte(REG_T, 24);
                }
                0xC5 => self.xC5(),
                0xC6 => self.xC6(),
                // RST: target = opcode & 0x38
                0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                    let pc = self.get_registry_value("PC");
                    self.tick_m();
                    self.push(pc);
                    self.store_result("PC", (opcode & 0x38) as u16, false);
                    self.regs.write_byte(REG_T, 16);
                }
                0xC9 => self.xC9(),
                0xCB => self.xCB(),
                0xCD => self.xCD(),
                0xCE => self.xCE(),
                0xD1 => self.xD1(),
                0xD5 => self.xD5(),
                0xD6 => self.xD6(),
                0xD9 => self.xD9(),
                0xDE => self.xDE(),
                0xE0 => self.xE0(),
                0xE1 => self.xE1(),
                0xE2 => self.xE2(),
                0xE5 => self.xE5(),
                0xE6 => self.xE6(),
                0xE8 => self.xE8(),
                0xE9 => self.xE9(),
                0xEA => self.xEA(),
                0xEE => self.xEE(),
                0xF0 => self.xF0(),
                0xF1 => self.xF1(),
                0xF2 => self.xF2(),
                0xF3 => self.xF3(),
                0xF5 => self.xF5(),
                0xF6 => self.xF6(),
                0xF8 => self.xF8(),
                0xF9 => self.xF9(),
                0xFA => self.xFA(),
                0xFB => self.xFB(),
                0xFE => self.xFE(),
                _ => {} // undefined/illegal opcodes
            }
        } else {
            self.execute_cb(opcode);
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
        self.tick_m();

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
        self.tick_m();

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
        self.tick_m();

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
        self.tick_m();

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

        let (_, _, _, prev_c) = self.regs.get_flags();

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

        self.tick_m();
        self.store_result("PC", result, false);

        self.regs.write_byte(REG_T, 12);
    }

    fn x19(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("DE");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);
        self.tick_m();

        self.regs.set_flags(old_z, false, h, c);

        self.regs.write_byte(REG_T, 8);
    }

    fn x1A(&mut self) {
        let op1 = self.get_operand_value("(DE)");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x1B(&mut self) {
        let op1 = self.get_operand_value("DE");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("DE", result, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn x1C(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x1D(&mut self) {
        let op1 = self.get_operand_value("E");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("E", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x1E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x1F(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let result = ((op1 as u8) >> 1 | (u8::from(prev_c) << 7)) as u16;
        let new_carry = (op1 & 1) != 0;

        self.store_result("A", result, true);

        self.regs.set_flags(false, false, false, new_carry);

        self.regs.write_byte(REG_T, 4);
    }

    fn x21(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("HL", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn x22(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_add(1), false);
        self.regs.write_byte(REG_T, 8);
    }

    fn x23(&mut self) {
        let op1 = self.get_operand_value("HL");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("HL", result, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn x24(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x25(&mut self) {
        let op1 = self.get_operand_value("H");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("H", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x26(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 8);
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
        self.regs
            .set_flags((result as u8) == 0, prev_n, false, new_carry);
        self.regs.write_byte(REG_T, 4);
    }

    fn x29(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("HL");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);
        self.tick_m();

        self.regs.set_flags(old_z, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn x2A(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_add(1), false);
        self.regs.write_byte(REG_T, 8);
    }

    fn x2B(&mut self) {
        let op1 = self.get_operand_value("HL");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("HL", result, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn x2C(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x2D(&mut self) {
        let op1 = self.get_operand_value("L");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("L", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x2E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x2F(&mut self) {
        let op1 = self.get_operand_value("A");
        let (z, _, _, c) = self.regs.get_flags();

        self.store_result("A", !op1, true);
        self.regs.set_flags(z, true, true, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x31(&mut self) {
        let op1 = self.get_operand_value("d16");
        self.store_result("SP", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn x32(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_sub(1), false);
        self.regs.write_byte(REG_T, 8);
    }

    fn x33(&mut self) {
        let op1 = self.get_operand_value("SP");

        let (result, _, _) = add_words(op1, 1, 0);

        self.store_result("SP", result, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn x34(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        self.regs.write_byte(REG_T, 12);
    }

    fn x35(&mut self) {
        let op1 = self.get_operand_value("(HL)");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("(HL)", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 12);
    }

    fn x36(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 12);
    }

    fn x37(&mut self) {
        let (z, _, _, _) = self.regs.get_flags();

        self.regs.set_flags(z, false, false, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x39(&mut self) {
        let op1 = self.get_operand_value("HL");
        let op2 = self.get_operand_value("SP");

        let (old_z, _, _, _) = self.regs.get_flags();

        let (result, c, h) = add_words(op1, op2, 0);

        self.store_result("HL", result, false);
        self.tick_m();

        self.regs.set_flags(old_z, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn x3A(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);

        let value = self.get_registry_value("HL");
        self.store_result("HL", value.wrapping_sub(1), false);
        self.regs.write_byte(REG_T, 8);
    }

    fn x3B(&mut self) {
        let op1 = self.get_operand_value("SP");

        let (result, _, _) = sub_bytes(op1, 1, 0);

        self.store_result("SP", result, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn x3C(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, prev_c) = self.regs.get_flags();

        let (result, _, h) = add_bytes(op1, 1, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x3D(&mut self) {
        let op1 = self.get_operand_value("A");

        let (_, _, _, c) = self.regs.get_flags();

        let (result, _, h) = sub_bytes(op1, 1, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x3E(&mut self) {
        let op1 = self.get_operand_value("d8");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x3F(&mut self) {
        let (z, _, _, c) = self.regs.get_flags();

        self.regs.set_flags(z, false, false, !c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x76(&mut self) {
        let pending = self.mmu.read_byte(0xFFFF) & self.mmu.read_byte(0xFF0F) & 0x1F;
        if !self.interrupt_master_enable && pending != 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }
        self.regs.write_byte(REG_T, 4);
    }

    fn xC1(&mut self) {
        let op1 = self.pop();
        self.store_result("BC", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xC3(&mut self) {
        let op1 = self.get_operand_value("a16");
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xC5(&mut self) {
        let op1 = self.get_operand_value("BC");
        self.tick_m();
        self.push(op1);
        self.regs.write_byte(REG_T, 16);
    }

    fn xC6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn xC9(&mut self) {
        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xCB(&mut self) {
        panic!("wtf?")
    }

    fn xCD(&mut self) {
        let op1 = self.get_operand_value("a16");

        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);

        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 24);
    }

    fn xCE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn xD1(&mut self) {
        let op1 = self.pop();
        self.store_result("DE", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xD5(&mut self) {
        let op1 = self.get_operand_value("DE");
        self.tick_m();
        self.push(op1);
        self.regs.write_byte(REG_T, 16);
    }

    fn xD6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn xD9(&mut self) {
        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);

        self.interrupt_master_enable = true;
        self.regs.write_byte(REG_T, 16);
    }

    fn xDE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn xE0(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(a8)", op1, true);
        self.regs.write_byte(REG_T, 12);
    }

    fn xE1(&mut self) {
        let op1 = self.pop();
        self.store_result("HL", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xE2(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(C)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn xE5(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.tick_m();
        self.push(op1);
        self.regs.write_byte(REG_T, 16);
    }

    fn xE6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xE8(&mut self) {
        let op1 = self.get_operand_value("SP");
        let op2 = self.get_operand_value("r8");

        let (result, c, h) = add_word_with_signed(op1, op2, 0);

        self.store_result("SP", result, false);
        self.tick_m();
        self.tick_m();

        self.regs.set_flags(false, false, h, c);
        self.regs.write_byte(REG_T, 16);
    }

    fn xE9(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xEA(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(a16)", op1, true);
        self.regs.write_byte(REG_T, 16);
    }

    fn xEE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xF0(&mut self) {
        let op1 = self.get_operand_value("(a8)");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 12);
    }

    fn xF1(&mut self) {
        let op1 = self.pop();
        self.store_result("AF", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xF2(&mut self) {
        let op1 = self.get_operand_value("(C)");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn xF3(&mut self) {
        self.interrupt_master_enable = false;
        self.regs.write_byte(REG_T, 4);
    }

    fn xF5(&mut self) {
        let op1 = self.get_operand_value("AF");
        self.tick_m();
        self.push(op1);
        self.regs.write_byte(REG_T, 16);
    }

    fn xF6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);

        self.regs.write_byte(REG_T, 8);
    }

    fn xF8(&mut self) {
        let op1 = self.get_operand_value("SP");
        let op2 = self.get_operand_value("r8");

        let (result, c, h) = add_word_with_signed(op1, op2, 0);

        self.store_result("HL", result, false);
        self.tick_m();

        self.regs.set_flags(false, false, h, c);
        self.regs.write_byte(REG_T, 12);
    }

    fn xF9(&mut self) {
        let op1 = self.get_operand_value("HL");
        self.store_result("SP", op1, false);
        self.tick_m();
        self.regs.write_byte(REG_T, 8);
    }

    fn xFA(&mut self) {
        let op1 = self.get_operand_value("(a16)");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 16);
    }

    fn xFB(&mut self) {
        self.schedule_interrupt_enable = true;
        self.regs.write_byte(REG_T, 4);
    }

    fn xFE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn cond_met(&mut self, opcode: u8) -> bool {
        let (z, _, _, c) = self.regs.get_flags();
        match (opcode >> 3) & 0x03 {
            0 => !z, // NZ
            1 => z,  // Z
            2 => !c, // NC
            3 => c,  // C
            _ => unreachable!(),
        }
    }

    fn cond_jr(&mut self, opcode: u8) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");
        if !self.cond_met(opcode) {
            self.regs.write_byte(REG_T, 8);
            return;
        }
        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;
        self.tick_m();
        self.store_result("PC", result, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn alu_a_r(&mut self, opcode: u8) {
        let src = Self::cb_reg_name(opcode & 0x07);
        let is_hl = (opcode & 0x07) == 6;
        let a = self.get_operand_value("A");
        let operand = self.get_operand_value(src);
        match (opcode >> 3) & 0x07 {
            0 => { // ADD
                let (result, c, h) = add_bytes(a, operand, 0);
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, false, h, c);
            }
            1 => { // ADC
                let (_, _, _, old_c) = self.regs.get_flags();
                let (result, c, h) = add_bytes(a, operand, if old_c { 1 } else { 0 });
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, false, h, c);
            }
            2 => { // SUB
                let (result, c, h) = sub_bytes(a, operand, 0);
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            3 => { // SBC
                let (_, _, _, old_c) = self.regs.get_flags();
                let (result, c, h) = sub_bytes(a, operand, if old_c { 1 } else { 0 });
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            4 => { // AND
                let result = a & operand;
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, false, true, false);
            }
            5 => { // XOR
                let result = a ^ operand;
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, false, false, false);
            }
            6 => { // OR
                let result = a | operand;
                self.store_result("A", result, true);
                self.regs.set_flags((result as u8) == 0, false, false, false);
            }
            7 => { // CP (compare: like SUB but discard result)
                let (result, c, h) = sub_bytes(a, operand, 0);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            _ => unreachable!(),
        }
        self.regs.write_byte(REG_T, if is_hl { 8 } else { 4 });
    }

    fn cb_reg_name(reg_idx: u8) -> &'static str {
        match reg_idx {
            0 => "B",
            1 => "C",
            2 => "D",
            3 => "E",
            4 => "H",
            5 => "L",
            6 => "(HL)",
            7 => "A",
            _ => unreachable!(),
        }
    }

    fn execute_cb(&mut self, opcode: u8) {
        let reg_idx = opcode & 0x07;
        let op_class = opcode >> 6;
        let sub_op = (opcode >> 3) & 0x07;
        let is_hl = reg_idx == 6;
        let reg = Self::cb_reg_name(reg_idx);

        match op_class {
            0 => {
                let op = self.get_operand_value(reg);
                let (_, _, _, prev_c) = self.regs.get_flags();
                let (result, new_carry) = match sub_op {
                    0 => cb_rlc(op),
                    1 => cb_rrc(op),
                    2 => cb_rl(op, prev_c),
                    3 => cb_rr(op, prev_c),
                    4 => cb_sla(op),
                    5 => cb_sra(op),
                    6 => cb_swap(op),
                    7 => cb_srl(op),
                    _ => unreachable!(),
                };
                self.store_result(reg, result, true);
                self.regs.set_flags((result as u8) == 0, false, false, new_carry);
                self.regs.write_byte(REG_T, if is_hl { 16 } else { 8 });
            }
            1 => {
                // BIT n, r
                let op = self.get_operand_value(reg);
                let (_, _, _, old_c) = self.regs.get_flags();
                let bit_set = is_bit_set(sub_op, op);
                self.regs.set_flags(!bit_set, false, true, old_c);
                self.regs.write_byte(REG_T, if is_hl { 12 } else { 8 });
            }
            2 => {
                // RES n, r
                let op = self.get_operand_value(reg);
                let result = reset_bit(sub_op, op as u8);
                self.store_result(reg, result, true);
                self.regs.write_byte(REG_T, if is_hl { 16 } else { 8 });
            }
            3 => {
                // SET n, r
                let op = self.get_operand_value(reg);
                let result = set_bit(sub_op, op as u8);
                self.store_result(reg, result, true);
                self.regs.write_byte(REG_T, if is_hl { 16 } else { 8 });
            }
            _ => unreachable!(),
        }
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

        assert_eq!(regs.read_byte(REG_A), 0x01);
        assert_eq!(regs.read_byte(REG_B), 0x00);
        assert_eq!(regs.read_byte(REG_C), 0x13);
        assert_eq!(regs.read_byte(REG_D), 0x00);
        assert_eq!(regs.read_byte(REG_E), 0xD8);
        assert_eq!(regs.read_byte(REG_H), 0x01);
        assert_eq!(regs.read_byte(REG_L), 0x4D);
        assert_eq!(regs.read_byte(REG_F), 0xB0);
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

        assert!(z);
        assert!(!n);
        assert!(h);
        assert!(!c);
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
