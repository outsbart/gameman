use crate::keypad::Button;

use crate::cartridge::load_rom;
use crate::cpu::{CPU, Operand};
use crate::gpu::GPU;
use crate::mem::{Interrupt, MMU, Memory};
use serde::{Deserialize, Serialize};
use std::io;

const CLOCKS_IN_A_FRAME: u32 = 70224;

#[derive(Serialize, Deserialize)]
pub struct Gameboy {
    pub cpu: CPU<MMU<GPU>>,
}

impl Gameboy {
    pub fn new(path: &str) -> Gameboy {
        let (cartridge, cgb_mode) = load_rom(path);
        let mmu = MMU::new(GPU::new(cgb_mode), cartridge);
        let mut cpu = CPU::new(mmu);
        if cgb_mode {
            // GBC hardware signals its presence via A=0x11 after the bootrom.
            // Games like Pokemon Yellow check this to decide whether to enable CGB features.
            cpu.write_reg(Operand::A, 0x11);
        }
        Gameboy { cpu }
    }

    pub fn load_bios(&mut self, path: &str) {
        let bytes = std::fs::read(path).expect("couldn't open boot rom");
        assert!(
            bytes.len() == 0x0100 || bytes.len() == 0x0900,
            "boot rom must be 256 (DMG) or 2304 (GBC) bytes"
        );
        let mut bios = [0u8; 0x0900];
        bios[..bytes.len()].copy_from_slice(&bytes);
        if bytes.len() == 0x0900 {
            if !self.cpu.mmu.gpu.cgb_mode {
                self.cpu.mmu.gpu.dmg_compat = true;
            }
            self.cpu.mmu.gpu.cgb_mode = true;
        }
        self.cpu.mmu.set_bios(bios);
        self.cpu.write_reg(Operand::PC, 0);
    }

    // fetch the operation, decodes it, and executes it.
    // returns the address of the executed instruction, the instruction opcode,
    // and t cycles passed during this step
    pub fn cpu_step(&mut self) -> (u16, u16, u8) {
        let line_number = self.cpu.read_reg(Operand::PC);

        let (instr, cycles_this_step) = self.cpu.step();

        let interrupt_cycles = self.cpu.handle_interrupts();

        (line_number, instr, cycles_this_step + interrupt_cycles)
    }

    pub fn step(&mut self) {
        let mut clocks_this_frame = 0u32;

        loop {
            let (_line, _opcode, t) = self.cpu_step();

            clocks_this_frame += u32::from(t);

            if clocks_this_frame >= CLOCKS_IN_A_FRAME {
                break;
            }
        }

        self.cpu.mmu.apply_gameshark_cheats();
    }

    /// Parse and store a single Game Genie / GameShark cheat code.
    pub fn add_cheat(&mut self, code: &str) -> Result<(), crate::cheats::CheatError> {
        self.cpu.mmu.add_cheat(code)
    }

    pub fn clear_cheats(&mut self) {
        self.cpu.mmu.clear_cheats();
    }

    pub fn get_framebuffer(&self) -> &[u32; 160 * 144] {
        self.cpu.mmu.gpu.get_buffer()
    }

    pub fn set_dmg_palette(&mut self, palette: [u32; 4]) {
        self.cpu.mmu.gpu.set_dmg_palette(palette);
    }

    pub fn drain_audio(&mut self) -> Vec<i16> {
        self.cpu.mmu.sound.drain_audio()
    }

    pub fn press_button(&mut self, button: Button) {
        self.cpu.mmu.key.press(button);
        self.request_keypad_interrupt();
    }

    pub fn release_button(&mut self, button: Button) {
        self.cpu.mmu.key.release(button);
    }

    /// Update MBC7 accelerometer axes. Call each frame before `step()`.
    /// x/y are centered at 0x8000; usable range ≈ 0x6000–0xA000 (±0x2000).
    /// No-op for non-MBC7 cartridges.
    pub fn set_accelerometer(&mut self, x: u16, y: u16) {
        self.cpu.mmu.cartridge.set_accel(x, y);
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        self.cpu.mmu.read_byte(addr)
    }

    pub fn get_link_buffer(&self) -> [char; 256] {
        self.cpu.mmu.link.get_buffer()
    }

    fn request_keypad_interrupt(&mut self) {
        self.cpu.mmu.request_interrupt(Interrupt::Joypad);
    }

    fn state_path(&self, slot: u8) -> std::path::PathBuf {
        self.cpu
            .mmu
            .cartridge
            .inner_cart()
            .path()
            .with_extension(format!("ss{slot}"))
    }

    pub fn save_state_to_file(&self, slot: u8) -> io::Result<()> {
        let bytes = bincode::serialize(self).map_err(io::Error::other)?;
        std::fs::write(self.state_path(slot), bytes)
    }

    pub fn load_state_from_file(&mut self, slot: u8) -> io::Result<()> {
        let path = self.state_path(slot);
        let bytes = std::fs::read(path)?;
        *self = bincode::deserialize(&bytes).map_err(io::Error::other)?;
        self.cpu.mmu.cartridge.restore()
    }
}
