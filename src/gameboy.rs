use crate::keypad::Button;

use crate::cartridge::load_rom;
use crate::cpu::CPU;
use crate::gpu::GPU;
use crate::mem::{MMU, Memory};
use crate::utils::load_boot_rom;

const CLOCKS_IN_A_FRAME: u32 = 70224;

pub struct Gameboy {
    pub(crate) cpu: CPU<MMU<GPU>>,
}

impl Gameboy {
    pub fn new(path: &str) -> Gameboy {
        let cartridge = load_rom(path);
        let mmu = MMU::new(GPU::new(), cartridge);
        let cpu = CPU::new(mmu);
        Gameboy { cpu }
    }

    pub fn load_bios(&mut self) {
        self.cpu.mmu.set_bios(load_boot_rom());
        self.cpu.set_registry_value("PC", 0);
    }

    // fetch the operation, decodes it, and executes it.
    // returns the address of the executed instruction, the instruction opcode,
    // and t cycles passed during this step
    pub fn cpu_step(&mut self) -> (u16, u16, u8) {
        let line_number = self.cpu.get_registry_value("PC");

        let (instr, cycles_this_step) = self.cpu.step();

        let interrupt_cycles = self.cpu.handle_interrupts();

        (line_number, instr, cycles_this_step + interrupt_cycles)
    }

    pub fn step(&mut self) {
        let mut clocks_this_frame = 0u32;

        loop {
            let (_line, _opcode, t) = self.cpu_step();

            clocks_this_frame += t as u32;

            if clocks_this_frame >= CLOCKS_IN_A_FRAME {
                break;
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u8; 160 * 144] {
        self.cpu.mmu.gpu.get_buffer()
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

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        self.cpu.mmu.read_byte(addr)
    }

    pub fn get_link_buffer(&self) -> [char; 256] {
        self.cpu.mmu.link.get_buffer()
    }

    pub fn get_cpu_register(&mut self, name: &str) -> u16 {
        self.cpu.get_registry_value(name)
    }

    fn request_keypad_interrupt(&mut self) {
        let interrupt_flags = self.cpu.mmu.read_byte(0xFF0F) | 0b10000;
        self.cpu.mmu.write_byte(0xFF0F, interrupt_flags);
    }
}
