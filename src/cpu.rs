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

    fn x20(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.tick_m();
        self.store_result("PC", result, false);
        self.regs.write_byte(REG_T, 12);
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

    fn x28(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("Z");

        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.tick_m();
        self.store_result("PC", result, false);
        self.regs.write_byte(REG_T, 12);
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

    fn x30(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.tick_m();
        self.store_result("PC", result, false);
        self.regs.write_byte(REG_T, 12);
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

    fn x38(&mut self) {
        let op1 = self.get_operand_value("PC");
        let op2 = self.get_operand_value("d8");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let result = (op1 as i16).wrapping_add(op2 as i8 as i16).wrapping_add(1) as u16;

        self.tick_m();
        self.store_result("PC", result, false);
        self.regs.write_byte(REG_T, 12);
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

    fn x40(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x41(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x42(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x43(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x44(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x45(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x46(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x47(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("B", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x48(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x49(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x4A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x4B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x4C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x4D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x4E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x4F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("C", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x50(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x51(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x52(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x53(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x54(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x55(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x56(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x57(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("D", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x58(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x59(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x5A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x5B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x5C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x5D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x5E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x5F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("E", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x60(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x61(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x62(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x63(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x64(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x65(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x66(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x67(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("H", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x68(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x69(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x6A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x6B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x6C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x6D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x6E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x6F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("L", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x70(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x71(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x72(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x73(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x74(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x75(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
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

    fn x77(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("(HL)", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x78(&mut self) {
        let op1 = self.get_operand_value("B");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x79(&mut self) {
        let op1 = self.get_operand_value("C");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x7A(&mut self) {
        let op1 = self.get_operand_value("D");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x7B(&mut self) {
        let op1 = self.get_operand_value("E");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x7C(&mut self) {
        let op1 = self.get_operand_value("H");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x7D(&mut self) {
        let op1 = self.get_operand_value("L");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x7E(&mut self) {
        let op1 = self.get_operand_value("(HL)");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x7F(&mut self) {
        let op1 = self.get_operand_value("A");
        self.store_result("A", op1, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x80(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x81(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x82(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x83(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x84(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x85(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x86(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn x87(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = add_bytes(op1, op2, 0);

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x88(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x89(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x8A(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x8B(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x8C(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x8D(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x8E(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn x8F(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (_, _, _, old_c) = self.regs.get_flags();

        let (result, c, h) = add_bytes(op1, op2, if old_c { 1 } else { 0 });

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn x90(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x91(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x92(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x93(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x94(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x95(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x96(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x97(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x98(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x99(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x9A(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x9B(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x9C(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x9D(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn x9E(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn x9F(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA0(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA1(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA2(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA3(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA4(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA5(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xA7(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 & op2;

        self.store_result("A", result, true);

        self.regs.set_flags((result as u8) == 0, false, true, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA8(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xA9(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xAA(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xAB(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xAC(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xAD(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xAE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xAF(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB0(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB1(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB2(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB3(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB4(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB5(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB6(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xB7(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let result = op1 | op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB8(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("B");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xB9(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("C");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xBA(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("D");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xBB(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("E");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xBC(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("H");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xBD(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("L");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xBE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("(HL)");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn xBF(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("A");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 4);
    }

    fn xC0(&mut self) {
        self.tick_m();
        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 20);
    }

    fn xC1(&mut self) {
        let op1 = self.pop();
        self.store_result("BC", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xC2(&mut self) {
        let op1 = self.get_operand_value("a16");
        let cond = self.get_operand_value("NZ");

        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xC3(&mut self) {
        let op1 = self.get_operand_value("a16");
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xC4(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NZ");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);

        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 24);
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

    fn xC7(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x00, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xC8(&mut self) {
        self.tick_m();
        let cond = self.get_operand_value("Z");

        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 20);
    }

    fn xC9(&mut self) {
        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xCA(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("Z");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xCB(&mut self) {
        panic!("wtf?")
    }

    fn xCC(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("Z");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);

        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 24);
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

    fn xCF(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x08, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xD0(&mut self) {
        self.tick_m();
        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 20);
    }

    fn xD1(&mut self) {
        let op1 = self.pop();
        self.store_result("DE", op1, false);
        self.regs.write_byte(REG_T, 12);
    }

    fn xD2(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xD3(&mut self) {}

    fn xD4(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("NC");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);

        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 24);
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

    fn xD7(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x10, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xD8(&mut self) {
        self.tick_m();
        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_T, 8);
            return;
        }

        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 20);
    }

    fn xD9(&mut self) {
        let op1 = self.pop();
        self.tick_m();
        self.store_result("PC", op1, false);

        self.interrupt_master_enable = true;
        self.regs.write_byte(REG_T, 16);
    }

    fn xDA(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        self.tick_m();
        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 16);
    }

    fn xDB(&mut self) {}

    fn xDC(&mut self) {
        let op1 = self.get_operand_value("a16");

        let cond = self.get_operand_value("CA");
        if cond == 0 {
            self.regs.write_byte(REG_T, 12);
            return;
        }

        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);

        self.store_result("PC", op1, false);
        self.regs.write_byte(REG_T, 24);
    }

    fn xDD(&mut self) {}

    fn xDE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");
        let (_, _, _, op3) = self.regs.get_flags();

        let (result, c, h) = sub_bytes(op1, op2, if op3 { 1 } else { 0 });

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.store_result("A", result, true);
        self.regs.write_byte(REG_T, 8);
    }

    fn xDF(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x18, false);
        self.regs.write_byte(REG_T, 16);
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

    fn xE3(&mut self) {}

    fn xE4(&mut self) {}

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

    fn xE7(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x20, false);
        self.regs.write_byte(REG_T, 16);
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

    fn xEB(&mut self) {}

    fn xEC(&mut self) {}

    fn xED(&mut self) {}

    fn xEE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let result = op1 ^ op2;

        self.store_result("A", result, true);

        self.regs
            .set_flags((result as u8) == 0, false, false, false);
        self.regs.write_byte(REG_T, 8);
    }

    fn xEF(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x28, false);
        self.regs.write_byte(REG_T, 16);
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

    fn xF4(&mut self) {}

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

    fn xF7(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x30, false);
        self.regs.write_byte(REG_T, 16);
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

    fn xFC(&mut self) {}

    fn xFD(&mut self) {}

    fn xFE(&mut self) {
        let op1 = self.get_operand_value("A");
        let op2 = self.get_operand_value("d8");

        let (result, c, h) = sub_bytes(op1, op2, 0);

        self.regs.set_flags((result as u8) == 0, true, h, c);
        self.regs.write_byte(REG_T, 8);
    }

    fn xFF(&mut self) {
        let value = self.get_registry_value("PC");
        self.tick_m();
        self.push(value);
        self.store_result("PC", 0x38, false);
        self.regs.write_byte(REG_T, 16);
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
