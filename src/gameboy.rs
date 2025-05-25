use crate::keypad::Button;

use crate::cartridge::load_rom;
use crate::cpu::CPU;
use crate::gpu::GPU;
use crate::mem::{MMU, Memory};
use crate::sound::AUDIO_BUFFER_SIZE;
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

    pub fn new_clean(path: &str) -> Gameboy {
        let sav_path = path.replacen(".gb", ".sav", 1);
        let _ = std::fs::remove_file(&sav_path);
        Self::new(path)
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

    pub fn get_audio_buffer(&mut self) -> Option<&[i16; AUDIO_BUFFER_SIZE]> {
        self.cpu.mmu.sound.get_audio_buffer()
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

    pub fn passes_test_rom(&mut self) -> bool {
        loop {
            self.step();

            let outbuffer = self.cpu.mmu.link.get_buffer();
            if outbuffer[0] != ' ' {
                let result: String = outbuffer.iter().collect();
                let passed: bool = result.contains("Passed");
                let failed: bool = result.contains("Failed");
                if passed {
                    return passed;
                }
                if failed {
                    return false;
                }
            }
        }
    }

    pub fn mooneye_step(&mut self) -> u8 {
        let mut clocks_this_frame = 0u32;

        // how many time was LD B,B executed?
        let mut ld_b_b: u8 = 0;

        loop {
            let (_line, opcode, t) = self.cpu_step();

            if opcode == 0x40 {
                ld_b_b += 1;
            }

            clocks_this_frame += t as u32;

            if clocks_this_frame >= CLOCKS_IN_A_FRAME {
                break;
            }
        }

        ld_b_b
    }

    pub fn passes_blargg_ram_test_rom(&mut self) -> bool {
        self.blargg_ram_test_result().map_or(false, |v| v == 0)
    }

    pub fn blargg_ram_test_result(&mut self) -> Option<u8> {
        let mut clocks = 0u32;
        loop {
            let (_line, _opcode, t) = self.cpu_step();
            clocks += t as u32;
            let val = self.cpu.mmu.read_byte(0xA000);
            match val {
                0x80 | 0xFF => {}
                v => return Some(v),
            }
            if clocks >= 3600 * 70224 {
                break;
            }
        }
        None
    }

    pub fn passes_mooneye_test_rom(&mut self) -> bool {
        let mut ld_b_b = 0;
        let mut frames = 0u32;

        loop {
            ld_b_b += self.mooneye_step();
            frames += 1;

            if ld_b_b > 1 {
                let b = self.cpu.get_registry_value("B");
                return b == 3;
            }

            if frames > 500 {
                return false;
            }
        }
    }

    fn request_keypad_interrupt(&mut self) {
        let interrupt_flags = self.cpu.mmu.read_byte(0xFF0F) | 0b10000;
        self.cpu.mmu.write_byte(0xFF0F, interrupt_flags);
    }
}
