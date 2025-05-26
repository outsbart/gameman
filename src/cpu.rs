#![allow(non_snake_case)]

use crate::mem::Memory;
use serde::{Deserialize, Serialize};
use crate::utils::add_bytes;
use crate::utils::add_word_with_signed;
use crate::utils::add_words;
use crate::utils::reset_bit;
use crate::utils::set_bit;
use crate::utils::sub_bytes;
use crate::utils::swap_nibbles;

pub const CPU_FREQ: usize = 4194304; // cpu frequency, in hz

// Flags bit position in the F register
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
const REG_PC: u16 = 10;

#[derive(Copy, Clone, Debug)]
pub enum Operand {
    // 8-bit registers
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    // 16-bit register pairs
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
    // Memory indirect via register
    IndBC,
    IndDE,
    IndHL,
    // High-RAM / special memory
    IndC,   // 0xFF00 + C
    IndA8,  // 0xFF00 + fetch_next_byte()
    IndA16, // fetch_next_word() as address
    // Immediates (read-only)
    D8,  // fetch_next_byte() as u16
    D16, // fetch_next_word()
}

#[derive(Serialize, Deserialize)]
struct Regs {
    regs: [u8; 12],
}

impl Regs {
    fn new() -> Regs {
        Regs { regs: [0; 12] }
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
    (
        ((val as u8) << 1 | u8::from(carry)) as u16,
        (val & 0x80) != 0,
    )
}
fn cb_rr(val: u16, carry: bool) -> (u16, bool) {
    (
        ((val as u8) >> 1 | (u8::from(carry) << 7)) as u16,
        (val & 1) != 0,
    )
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

#[derive(Serialize, Deserialize)]
#[serde(bound = "M: Serialize + for<'de2> serde::Deserialize<'de2>")]
pub struct CPU<M: Memory> {
    regs: Regs,
    pub mmu: M,
    interrupt_master_enable: bool,
    schedule_interrupt_enable: bool,
    stopped: bool,
    halted: bool,
    halt_bug: bool,
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

    fn reset(&mut self) {
        self.write_reg(Operand::AF, 0x01B0);
        self.write_reg(Operand::BC, 0x0013);
        self.write_reg(Operand::DE, 0x00D8);
        self.write_reg(Operand::HL, 0x014D);
        self.write_reg(Operand::SP, 0xFFFE);
        self.write_reg(Operand::PC, 0x100);
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

    pub fn read_reg(&mut self, r: Operand) -> u16 {
        match r {
            Operand::A => self.regs.read_byte(REG_A) as u16,
            Operand::F => self.regs.read_byte(REG_F) as u16,
            Operand::B => self.regs.read_byte(REG_B) as u16,
            Operand::C => self.regs.read_byte(REG_C) as u16,
            Operand::D => self.regs.read_byte(REG_D) as u16,
            Operand::E => self.regs.read_byte(REG_E) as u16,
            Operand::H => self.regs.read_byte(REG_H) as u16,
            Operand::L => self.regs.read_byte(REG_L) as u16,
            Operand::AF => self.regs.read_word(REG_A),
            Operand::BC => self.regs.read_word(REG_B),
            Operand::DE => self.regs.read_word(REG_D),
            Operand::HL => self.regs.read_word(REG_H),
            Operand::SP => self.regs.read_word(REG_SP),
            Operand::PC => self.regs.read_word(REG_PC),
            _ => unreachable!(),
        }
    }

    pub fn write_reg(&mut self, r: Operand, val: u16) {
        match r {
            Operand::A => self.regs.write_byte(REG_A, val as u8),
            Operand::F => self.regs.write_byte(REG_F, val as u8),
            Operand::B => self.regs.write_byte(REG_B, val as u8),
            Operand::C => self.regs.write_byte(REG_C, val as u8),
            Operand::D => self.regs.write_byte(REG_D, val as u8),
            Operand::E => self.regs.write_byte(REG_E, val as u8),
            Operand::H => self.regs.write_byte(REG_H, val as u8),
            Operand::L => self.regs.write_byte(REG_L, val as u8),
            Operand::AF => self.regs.write_word(REG_A, val),
            Operand::BC => self.regs.write_word(REG_B, val),
            Operand::DE => self.regs.write_word(REG_D, val),
            Operand::HL => self.regs.write_word(REG_H, val),
            Operand::SP => self.regs.write_word(REG_SP, val),
            Operand::PC => self.regs.write_word(REG_PC, val),
            _ => unreachable!(),
        }
    }

    pub fn read_operand(&mut self, op: Operand) -> u16 {
        match op {
            Operand::A
            | Operand::B
            | Operand::C
            | Operand::D
            | Operand::E
            | Operand::H
            | Operand::L
            | Operand::AF
            | Operand::BC
            | Operand::DE
            | Operand::HL
            | Operand::SP
            | Operand::PC => self.read_reg(op),
            Operand::IndBC => {
                let addr = self.read_reg(Operand::BC);
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::IndDE => {
                let addr = self.read_reg(Operand::DE);
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::IndHL => {
                let addr = self.read_reg(Operand::HL);
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::IndC => {
                let addr = 0xFF00 + self.read_reg(Operand::C);
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::IndA8 => {
                let addr = 0xFF00 + u16::from(self.fetch_next_byte());
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::IndA16 => {
                let addr = self.fetch_next_word();
                let val = self.mmu.read_byte(addr) as u16;
                self.tick_m();
                val
            }
            Operand::D8 => self.fetch_next_byte() as u16,
            Operand::D16 => self.fetch_next_word(),
            Operand::F => unreachable!(),
        }
    }

    pub fn write_operand(&mut self, dst: Operand, val: u16) {
        let addr: u16 = match dst {
            Operand::A
            | Operand::B
            | Operand::C
            | Operand::D
            | Operand::E
            | Operand::H
            | Operand::L
            | Operand::AF
            | Operand::BC
            | Operand::DE
            | Operand::HL
            | Operand::SP
            | Operand::PC => {
                self.write_reg(dst, val);
                return;
            }
            Operand::IndBC => self.read_reg(Operand::BC),
            Operand::IndDE => self.read_reg(Operand::DE),
            Operand::IndHL => self.read_reg(Operand::HL),
            Operand::IndC => 0xFF00 + self.read_reg(Operand::C),
            Operand::IndA8 => 0xFF00 + u16::from(self.fetch_next_byte()),
            Operand::IndA16 => self.fetch_next_word(),
            _ => unreachable!(),
        };
        self.mmu.write_byte(addr, val as u8);
        self.tick_m();
    }

    pub fn push(&mut self, value: u16) {
        let sp = self.read_reg(Operand::SP);
        self.mmu
            .write_byte(sp.wrapping_sub(1), ((value >> 8) & 0xFF) as u8);
        self.tick_m();
        self.mmu
            .write_byte(sp.wrapping_sub(2), (value & 0xFF) as u8);
        self.tick_m();
        self.write_reg(Operand::SP, sp.wrapping_sub(2));
    }

    pub fn pop(&mut self) -> u16 {
        let sp = self.read_reg(Operand::SP);
        let low = self.mmu.read_byte(sp) as u16;
        self.tick_m();
        let high = self.mmu.read_byte(sp.wrapping_add(1)) as u16;
        self.tick_m();
        self.write_reg(Operand::SP, sp.wrapping_add(2));
        low | (high << 8)
    }

    // executes the next instruction
    // returns instruction executed and cycles taken
    pub fn step(&mut self) -> (u16, u8) {
        let mut instr: u16 = 0;

        let cycles = if !self.halted {
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
                    0x03 | 0x0B => Some(self.read_reg(Operand::BC)),
                    0x13 | 0x1B => Some(self.read_reg(Operand::DE)),
                    0x23 | 0x2B | 0x2A | 0x3A => Some(self.read_reg(Operand::HL)),
                    0x33 | 0x3B => Some(self.read_reg(Operand::SP)),
                    0xC1 | 0xD1 | 0xE1 | 0xF1 | 0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                        Some(self.read_reg(Operand::SP))
                    }
                    _ => None,
                }
            } else {
                None
            };

            self.enable_ime_if_scheduled();
            let cycles = self.execute(byte, prefixed);

            if let Some(rr) = oam_rr {
                self.mmu.handle_oam_corruption(byte, rr);
            }

            cycles
        } else {
            self.tick_m();
            4
        };

        (instr, cycles)
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
            let pc = self.read_reg(Operand::PC);
            let sp = self.read_reg(Operand::SP);
            self.mmu
                .write_byte(sp.wrapping_sub(1), ((pc >> 8) & 0xFF) as u8);
            self.tick_m();

            // Hardware re-reads IE & IF after M3 to determine the vector.
            // If IE changed during M3 (e.g. an ISR wrote to it), the new value wins.
            // If pending becomes 0, dispatch is "cancelled" and PC lands at $0000.
            let pending = self.interrupts_to_handle();

            // M4: push PC low byte
            self.mmu.write_byte(sp.wrapping_sub(2), (pc & 0xFF) as u8);
            self.tick_m();
            self.write_reg(Operand::SP, sp.wrapping_sub(2));

            // M5: load vector
            self.tick_m();

            let interrupt_flags = self.mmu.read_byte(0xFF0F);

            if (pending & 0x01) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(0, interrupt_flags) as u8);
                self.write_reg(Operand::PC, 0x0040);
            } else if (pending & 0x02) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(1, interrupt_flags) as u8);
                self.write_reg(Operand::PC, 0x0048);
            } else if (pending & 0x04) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(2, interrupt_flags) as u8);
                self.write_reg(Operand::PC, 0x0050);
            } else if (pending & 0x08) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(3, interrupt_flags) as u8);
                self.write_reg(Operand::PC, 0x0058);
            } else if (pending & 0x10) != 0 {
                self.mmu
                    .write_byte(0xFF0F, reset_bit(4, interrupt_flags) as u8);
                self.write_reg(Operand::PC, 0x0060);
            } else {
                // All bits cleared before vector load — PC becomes $0000
                self.write_reg(Operand::PC, 0x0000);
            }

            return 20;
        }

        0
    }

    pub fn execute(&mut self, opcode: u8, cb: bool) -> u8 {
        if !cb {
            match opcode {
                0x00 => self.nop(),
                // LD r16, d16: r16 = bits 5-4
                0x01 | 0x11 | 0x21 | 0x31 => self.ld_r16_d16(opcode),
                // LD (r16), A: with HL+/HL- post-modify for 0x22/0x32
                0x02 | 0x12 | 0x22 | 0x32 => self.ld_ind_r16_a(opcode),
                // INC r16: r16 = bits 5-4
                0x03 | 0x13 | 0x23 | 0x33 => self.inc_r16(opcode),
                // INC r8: r8 = bits 5-3
                0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => self.inc_r8(opcode),
                // DEC r8: r8 = bits 5-3
                0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => self.dec_r8(opcode),
                // LD r8, d8: r8 = bits 5-3
                0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => self.ld_r8_d8(opcode),
                // RLCA/RRCA/RLA/RRA: op = bits 4-3
                0x07 | 0x0F | 0x17 | 0x1F => self.rotate_a(opcode),
                0x08 => self.ld_ind_a16_sp(),
                // ADD HL, r16: r16 = bits 5-4
                0x09 | 0x19 | 0x29 | 0x39 => self.add_hl_r16(opcode),
                // LD A, (r16): with HL+/HL- post-modify for 0x2A/0x3A
                0x0A | 0x1A | 0x2A | 0x3A => self.ld_a_ind_r16(opcode),
                // DEC r16: r16 = bits 5-4
                0x0B | 0x1B | 0x2B | 0x3B => self.dec_r16(opcode),
                0x10 => self.stop(),
                0x18 => self.jr_e(),
                0x20 | 0x28 | 0x30 | 0x38 => self.cond_jr(opcode),
                0x27 => self.daa(),
                0x2F => self.cpl(),
                0x37 => self.scf(),
                0x3F => self.ccf(),
                // LD r, r': dst = bits 5-3, src = bits 2-0
                0x40..=0x75 | 0x77..=0x7F => self.ld_r_r(opcode),
                0x76 => self.halt(),
                // ALU A, r: op = bits 5-3, src = bits 2-0
                0x80..=0xBF => self.alu_a_r(opcode),
                // RET cc
                0xC0 | 0xC8 | 0xD0 | 0xD8 => self.ret_cc(opcode),
                // POP r16: r16 = bits 5-4 (BC/DE/HL/AF)
                0xC1 | 0xD1 | 0xE1 | 0xF1 => self.pop_r16(opcode),
                // JP cc, a16
                0xC2 | 0xCA | 0xD2 | 0xDA => self.jp_cc(opcode),
                0xC3 => self.jp_a16(),
                // CALL cc, a16
                0xC4 | 0xCC | 0xD4 | 0xDC => self.call_cc(opcode),
                // PUSH r16: r16 = bits 5-4 (BC/DE/HL/AF)
                0xC5 | 0xD5 | 0xE5 | 0xF5 => self.push_r16(opcode),
                // ALU A, d8: op = bits 5-3
                0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.alu_a_imm8(opcode),
                // RST: target = opcode & 0x38
                0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.rst(opcode),
                0xC9 => self.ret(),
                0xCB => self.xCB(),
                0xCD => self.call_a16(),
                0xD9 => self.reti(),
                0xE0 => self.ldh_ind_a8_a(),
                0xE2 => self.ld_ind_c_a(),
                0xE8 => self.add_sp_e(),
                0xE9 => self.jp_hl(),
                0xEA => self.ld_ind_a16_a(),
                0xF0 => self.ldh_a_ind_a8(),
                0xF2 => self.ld_a_ind_c(),
                0xF3 => self.di(),
                0xF8 => self.ld_hl_sp_e(),
                0xF9 => self.ld_sp_hl(),
                0xFA => self.ld_a_ind_a16(),
                0xFB => self.ei(),
                _ => 0, // undefined/illegal opcodes
            }
        } else {
            self.execute_cb(opcode)
        }
    }

    // === Control ===

    fn nop(&mut self) -> u8 {
        4
    }

    fn stop(&mut self) -> u8 {
        self.stopped = true;
        4
    }

    fn halt(&mut self) -> u8 {
        let pending = self.mmu.read_byte(0xFFFF) & self.mmu.read_byte(0xFF0F) & 0x1F;
        if !self.interrupt_master_enable && pending != 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }
        4
    }

    fn di(&mut self) -> u8 {
        self.interrupt_master_enable = false;
        4
    }

    fn ei(&mut self) -> u8 {
        self.schedule_interrupt_enable = true;
        4
    }

    // === Loads (8-bit) ===

    fn ld_r_r(&mut self, opcode: u8) -> u8 {
        let src = Self::cb_reg(opcode & 0x07);
        let dst = Self::cb_reg((opcode >> 3) & 0x07);
        let val = self.read_operand(src);
        self.write_operand(dst, val);
        let is_hl = (opcode & 0x07) == 6 || ((opcode >> 3) & 0x07) == 6;
        if is_hl { 8 } else { 4 }
    }

    fn ld_r8_d8(&mut self, opcode: u8) -> u8 {
        let idx = (opcode >> 3) & 0x07;
        let reg = Self::cb_reg(idx);
        let val = self.read_operand(Operand::D8);
        self.write_operand(reg, val);
        if idx == 6 { 12 } else { 8 }
    }

    fn ld_ind_r16_a(&mut self, opcode: u8) -> u8 {
        let a = self.read_reg(Operand::A);
        match (opcode >> 4) & 0x03 {
            0 => self.write_operand(Operand::IndBC, a),
            1 => self.write_operand(Operand::IndDE, a),
            2 => {
                self.write_operand(Operand::IndHL, a);
                let hl = self.read_reg(Operand::HL);
                self.write_reg(Operand::HL, hl.wrapping_add(1));
            }
            3 => {
                self.write_operand(Operand::IndHL, a);
                let hl = self.read_reg(Operand::HL);
                self.write_reg(Operand::HL, hl.wrapping_sub(1));
            }
            _ => unreachable!(),
        }
        8
    }

    fn ld_a_ind_r16(&mut self, opcode: u8) -> u8 {
        let val = match (opcode >> 4) & 0x03 {
            0 => self.read_operand(Operand::IndBC),
            1 => self.read_operand(Operand::IndDE),
            2 => {
                let v = self.read_operand(Operand::IndHL);
                let hl = self.read_reg(Operand::HL);
                self.write_reg(Operand::HL, hl.wrapping_add(1));
                v
            }
            3 => {
                let v = self.read_operand(Operand::IndHL);
                let hl = self.read_reg(Operand::HL);
                self.write_reg(Operand::HL, hl.wrapping_sub(1));
                v
            }
            _ => unreachable!(),
        };
        self.write_reg(Operand::A, val);
        8
    }

    fn ldh_ind_a8_a(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::A);
        self.write_operand(Operand::IndA8, op1);
        12
    }

    fn ldh_a_ind_a8(&mut self) -> u8 {
        let op1 = self.read_operand(Operand::IndA8);
        self.write_reg(Operand::A, op1);
        12
    }

    fn ld_ind_c_a(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::A);
        self.write_operand(Operand::IndC, op1);
        8
    }

    fn ld_a_ind_c(&mut self) -> u8 {
        let op1 = self.read_operand(Operand::IndC);
        self.write_reg(Operand::A, op1);
        8
    }

    fn ld_ind_a16_a(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::A);
        self.write_operand(Operand::IndA16, op1);
        16
    }

    fn ld_a_ind_a16(&mut self) -> u8 {
        let op1 = self.read_operand(Operand::IndA16);
        self.write_reg(Operand::A, op1);
        16
    }

    fn ld_ind_a16_sp(&mut self) -> u8 {
        let addr = self.fetch_next_word();
        let sp = self.read_reg(Operand::SP);
        self.mmu.write_byte(addr, (sp & 0xFF) as u8);
        self.tick_m();
        self.mmu.write_byte(addr.wrapping_add(1), ((sp >> 8) & 0xFF) as u8);
        self.tick_m();
        20
    }

    // === Loads (16-bit) ===

    fn ld_r16_d16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16((opcode >> 4) & 0x03);
        let val = self.read_operand(Operand::D16);
        self.write_reg(reg, val);
        12
    }

    fn ld_sp_hl(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::HL);
        self.write_reg(Operand::SP, op1);
        self.tick_m();
        8
    }

    fn ld_hl_sp_e(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::SP);
        let op2 = self.read_operand(Operand::D8);
        let (result, c, h) = add_word_with_signed(op1, op2, 0);
        self.write_reg(Operand::HL, result);
        self.tick_m();
        self.regs.set_flags(false, false, h, c);
        12
    }

    // === ALU (8-bit) ===

    fn alu_a_r(&mut self, opcode: u8) -> u8 {
        let src = Self::cb_reg(opcode & 0x07);
        let is_hl = (opcode & 0x07) == 6;
        let a = self.read_reg(Operand::A);
        let operand = self.read_operand(src);
        self.alu_apply((opcode >> 3) & 0x07, a, operand);
        if is_hl { 8 } else { 4 }
    }

    fn alu_a_imm8(&mut self, opcode: u8) -> u8 {
        let a = self.read_reg(Operand::A);
        let imm = self.read_operand(Operand::D8);
        self.alu_apply((opcode >> 3) & 0x07, a, imm);
        8
    }

    fn alu_apply(&mut self, op_idx: u8, a: u16, operand: u16) {
        match op_idx {
            0 => {
                // ADD
                let (result, c, h) = add_bytes(a, operand, 0);
                self.write_reg(Operand::A, result);
                self.regs.set_flags((result as u8) == 0, false, h, c);
            }
            1 => {
                // ADC
                let (_, _, _, old_c) = self.regs.get_flags();
                let (result, c, h) = add_bytes(a, operand, if old_c { 1 } else { 0 });
                self.write_reg(Operand::A, result);
                self.regs.set_flags((result as u8) == 0, false, h, c);
            }
            2 => {
                // SUB
                let (result, c, h) = sub_bytes(a, operand, 0);
                self.write_reg(Operand::A, result);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            3 => {
                // SBC
                let (_, _, _, old_c) = self.regs.get_flags();
                let (result, c, h) = sub_bytes(a, operand, if old_c { 1 } else { 0 });
                self.write_reg(Operand::A, result);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            4 => {
                // AND
                let result = a & operand;
                self.write_reg(Operand::A, result);
                self.regs.set_flags((result as u8) == 0, false, true, false);
            }
            5 => {
                // XOR
                let result = a ^ operand;
                self.write_reg(Operand::A, result);
                self.regs
                    .set_flags((result as u8) == 0, false, false, false);
            }
            6 => {
                // OR
                let result = a | operand;
                self.write_reg(Operand::A, result);
                self.regs
                    .set_flags((result as u8) == 0, false, false, false);
            }
            7 => {
                // CP
                let (result, c, h) = sub_bytes(a, operand, 0);
                self.regs.set_flags((result as u8) == 0, true, h, c);
            }
            _ => unreachable!(),
        }
    }

    fn inc_r8(&mut self, opcode: u8) -> u8 {
        let idx = (opcode >> 3) & 0x07;
        let reg = Self::cb_reg(idx);
        let (_, _, _, prev_c) = self.regs.get_flags();
        let op1 = self.read_operand(reg);
        let (result, _, h) = add_bytes(op1, 1, 0);
        self.write_operand(reg, result);
        self.regs.set_flags((result as u8) == 0, false, h, prev_c);
        if idx == 6 { 12 } else { 4 }
    }

    fn dec_r8(&mut self, opcode: u8) -> u8 {
        let idx = (opcode >> 3) & 0x07;
        let reg = Self::cb_reg(idx);
        let (_, _, _, prev_c) = self.regs.get_flags();
        let op1 = self.read_operand(reg);
        let (result, _, h) = sub_bytes(op1, 1, 0);
        self.write_operand(reg, result);
        self.regs.set_flags((result as u8) == 0, true, h, prev_c);
        if idx == 6 { 12 } else { 4 }
    }

    fn daa(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::A);
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

        self.write_reg(Operand::A, result);
        self.regs
            .set_flags((result as u8) == 0, prev_n, false, new_carry);
        4
    }

    fn cpl(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::A);
        let (z, _, _, c) = self.regs.get_flags();
        self.write_reg(Operand::A, !op1);
        self.regs.set_flags(z, true, true, c);
        4
    }

    fn scf(&mut self) -> u8 {
        let (z, _, _, _) = self.regs.get_flags();
        self.regs.set_flags(z, false, false, true);
        4
    }

    fn ccf(&mut self) -> u8 {
        let (z, _, _, c) = self.regs.get_flags();
        self.regs.set_flags(z, false, false, !c);
        4
    }

    // === ALU (16-bit) ===

    fn add_hl_r16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16((opcode >> 4) & 0x03);
        let hl = self.read_reg(Operand::HL);
        let op2 = self.read_reg(reg);
        let (old_z, _, _, _) = self.regs.get_flags();
        let (result, c, h) = add_words(hl, op2, 0);
        self.write_reg(Operand::HL, result);
        self.tick_m();
        self.regs.set_flags(old_z, false, h, c);
        8
    }

    fn inc_r16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16((opcode >> 4) & 0x03);
        let op1 = self.read_reg(reg);
        let (result, _, _) = add_words(op1, 1, 0);
        self.write_reg(reg, result);
        self.tick_m();
        8
    }

    fn dec_r16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16((opcode >> 4) & 0x03);
        let op1 = self.read_reg(reg);
        let (result, _, _) = sub_bytes(op1, 1, 0);
        self.write_reg(reg, result);
        self.tick_m();
        8
    }

    fn add_sp_e(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::SP);
        let op2 = self.read_operand(Operand::D8);
        let (result, c, h) = add_word_with_signed(op1, op2, 0);
        self.write_reg(Operand::SP, result);
        self.tick_m();
        self.tick_m();
        self.regs.set_flags(false, false, h, c);
        16
    }

    // === Rotates ===

    fn rotate_a(&mut self, opcode: u8) -> u8 {
        let op = self.read_reg(Operand::A);
        let (_, _, _, prev_c) = self.regs.get_flags();
        let (result, new_carry) = match (opcode >> 3) & 0x03 {
            0 => {
                // RLCA
                let c = (op & 0x80) != 0;
                (((op as u8) << 1 | u8::from(c)) as u16, c)
            }
            1 => {
                // RRCA
                let c = (op & 1) != 0;
                (((op as u8) >> 1 | (u8::from(c) << 7)) as u16, c)
            }
            2 => {
                // RLA
                let c = (op & 0x80) != 0;
                (((op as u8) << 1 | u8::from(prev_c)) as u16, c)
            }
            3 => {
                // RRA
                let c = (op & 1) != 0;
                (((op as u8) >> 1 | (u8::from(prev_c) << 7)) as u16, c)
            }
            _ => unreachable!(),
        };
        self.write_reg(Operand::A, result);
        self.regs.set_flags(false, false, false, new_carry);
        4
    }

    // === Jumps / Branches ===

    fn jr_e(&mut self) -> u8 {
        let offset = self.read_operand(Operand::D8);
        self.jr_impl(offset);
        12
    }

    fn cond_jr(&mut self, opcode: u8) -> u8 {
        let offset = self.read_operand(Operand::D8);
        if !self.cond_met(opcode) {
            return 8;
        }
        self.jr_impl(offset);
        12
    }

    fn jr_impl(&mut self, offset: u16) {
        let pc = self.read_reg(Operand::PC);
        let result = pc.wrapping_add(offset as i8 as u16);
        self.tick_m();
        self.write_reg(Operand::PC, result);
    }

    fn jp_a16(&mut self) -> u8 {
        let addr = self.read_operand(Operand::D16);
        self.jp_impl(addr);
        16
    }

    fn jp_cc(&mut self, opcode: u8) -> u8 {
        let addr = self.read_operand(Operand::D16);
        if !self.cond_met(opcode) {
            return 12;
        }
        self.jp_impl(addr);
        16
    }

    fn jp_hl(&mut self) -> u8 {
        let op1 = self.read_reg(Operand::HL);
        self.write_reg(Operand::PC, op1);
        4
    }

    fn jp_impl(&mut self, addr: u16) {
        self.tick_m();
        self.write_reg(Operand::PC, addr);
    }

    fn call_a16(&mut self) -> u8 {
        let addr = self.read_operand(Operand::D16);
        self.call_impl(addr);
        24
    }

    fn call_cc(&mut self, opcode: u8) -> u8 {
        let addr = self.read_operand(Operand::D16);
        if !self.cond_met(opcode) {
            return 12;
        }
        self.call_impl(addr);
        24
    }

    fn call_impl(&mut self, addr: u16) {
        let pc = self.read_reg(Operand::PC);
        self.tick_m();
        self.push(pc);
        self.write_reg(Operand::PC, addr);
    }

    fn ret(&mut self) -> u8 {
        let addr = self.ret_impl();
        self.write_reg(Operand::PC, addr);
        16
    }

    fn ret_cc(&mut self, opcode: u8) -> u8 {
        self.tick_m();
        if !self.cond_met(opcode) {
            return 8;
        }
        let addr = self.ret_impl();
        self.write_reg(Operand::PC, addr);
        20
    }

    fn ret_impl(&mut self) -> u16 {
        let addr = self.pop();
        self.tick_m();
        addr
    }

    fn reti(&mut self) -> u8 {
        let addr = self.ret_impl();
        self.write_reg(Operand::PC, addr);
        self.interrupt_master_enable = true;
        16
    }

    fn rst(&mut self, opcode: u8) -> u8 {
        let pc = self.read_reg(Operand::PC);
        self.tick_m();
        self.push(pc);
        self.write_reg(Operand::PC, (opcode & 0x38) as u16);
        16
    }

    // === Stack ===

    fn push_r16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16_af((opcode >> 4) & 0x03);
        let val = self.read_reg(reg);
        self.tick_m();
        self.push(val);
        16
    }

    fn pop_r16(&mut self, opcode: u8) -> u8 {
        let reg = Self::r16_af((opcode >> 4) & 0x03);
        let val = self.pop();
        self.write_reg(reg, val);
        12
    }

    // === Helpers ===

    fn cb_reg(reg_idx: u8) -> Operand {
        match reg_idx {
            0 => Operand::B,
            1 => Operand::C,
            2 => Operand::D,
            3 => Operand::E,
            4 => Operand::H,
            5 => Operand::L,
            6 => Operand::IndHL,
            7 => Operand::A,
            _ => unreachable!(),
        }
    }

    fn r16(idx: u8) -> Operand {
        match idx {
            0 => Operand::BC,
            1 => Operand::DE,
            2 => Operand::HL,
            3 => Operand::SP,
            _ => unreachable!(),
        }
    }

    fn r16_af(idx: u8) -> Operand {
        match idx {
            0 => Operand::BC,
            1 => Operand::DE,
            2 => Operand::HL,
            3 => Operand::AF,
            _ => unreachable!(),
        }
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

    fn xCB(&mut self) -> u8 {
        panic!("wtf?")
    }

    fn execute_cb(&mut self, opcode: u8) -> u8 {
        let reg_idx = opcode & 0x07;
        let op_class = opcode >> 6;
        let sub_op = (opcode >> 3) & 0x07;
        let is_hl = reg_idx == 6;
        let reg = Self::cb_reg(reg_idx);

        match op_class {
            0 => {
                let op = self.read_operand(reg);
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
                self.write_operand(reg, result);
                self.regs
                    .set_flags((result as u8) == 0, false, false, new_carry);
                if is_hl { 16 } else { 8 }
            }
            1 => {
                // BIT n, r
                let op = self.read_operand(reg);
                let (_, _, _, old_c) = self.regs.get_flags();
                let bit_set = is_bit_set(sub_op, op);
                self.regs.set_flags(!bit_set, false, true, old_c);
                if is_hl { 12 } else { 8 }
            }
            2 => {
                // RES n, r
                let op = self.read_operand(reg);
                let result = reset_bit(sub_op, op as u8);
                self.write_operand(reg, result);
                if is_hl { 16 } else { 8 }
            }
            3 => {
                // SET n, r
                let op = self.read_operand(reg);
                let result = set_bit(sub_op, op as u8);
                self.write_operand(reg, result);
                if is_hl { 16 } else { 8 }
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
        let CPU { mut regs, .. } = CPU::new(DummyMMU::new());

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

        cpu.write_reg(Operand::PC, 500);
        cpu.mmu.values[500] = 0x18;
        cpu.mmu.values[501] = 0b0000_0010; // jump by 2

        cpu.step();

        assert_eq!(cpu.read_reg(Operand::PC), 504);
    }

    #[test]
    fn test_jr_negative() {
        let mut cpu = CPU::new(DummyMMU::new());

        cpu.write_reg(Operand::PC, 500);
        cpu.mmu.values[500] = 0x18;
        cpu.mmu.values[501] = 0b1111_1110; // jump by -2

        cpu.step();

        assert_eq!(cpu.read_reg(Operand::PC), 500);
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

        // set next instruction to POP AF
        cpu.write_reg(Operand::PC, 500);
        cpu.mmu.values[500] = 0xF1;

        // execute it
        cpu.step();

        // lower nibble of F must be untouched
        assert_eq!(cpu.read_reg(Operand::F), 0xF0)
    }
}
