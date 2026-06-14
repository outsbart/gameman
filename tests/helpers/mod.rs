#![allow(dead_code, unused_macros)]

use gameman::cpu::Reg8;
use gameman::gameboy::Gameboy;

const CLOCKS_IN_A_FRAME: u32 = 70224;

pub trait GameboyTestExt {
    fn new_clean(path: &str) -> Gameboy;
    fn passes_test_rom(&mut self) -> bool;
    fn passes_blargg_ram_test_rom(&mut self) -> bool;
    fn blargg_ram_test_result(&mut self) -> Option<u8>;
    fn passes_mooneye_test_rom(&mut self) -> bool;
    fn mooneye_step(&mut self) -> u8;
}

macro_rules! test_rom {
    ($(#[$attr:meta])* $name:ident, $path:literal, $method:ident) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            let mut emulator = Gameboy::new($path);
            assert!(emulator.$method());
        }
    };
    ($(#[$attr:meta])* $name:ident, $path:literal, $method:ident, new_clean) => {
        $(#[$attr])*
        #[test]
        fn $name() {
            let mut emulator = Gameboy::new_clean($path);
            assert!(emulator.$method());
        }
    };
}

impl GameboyTestExt for Gameboy {
    fn new_clean(path: &str) -> Gameboy {
        let sav_path = path.replacen(".gb", ".sav", 1);
        let _ = std::fs::remove_file(&sav_path);
        Gameboy::new(path)
    }

    fn passes_test_rom(&mut self) -> bool {
        loop {
            self.step();

            let outbuffer = self.get_link_buffer();
            if outbuffer[0] != ' ' {
                let result: String = outbuffer.iter().collect();
                let passed = result.contains("Passed");
                let failed = result.contains("Failed");
                if passed {
                    return true;
                }
                if failed {
                    return false;
                }
            }
        }
    }

    fn passes_blargg_ram_test_rom(&mut self) -> bool {
        self.blargg_ram_test_result() == Some(0)
    }

    fn blargg_ram_test_result(&mut self) -> Option<u8> {
        let mut clocks = 0u32;
        loop {
            let (_line, _opcode, t) = self.cpu_step();
            clocks += u32::from(t);
            let val = self.read_byte(0xA000);
            match val {
                0x80 | 0xFF => {}
                v => return Some(v),
            }
            if clocks >= 3600 * CLOCKS_IN_A_FRAME {
                break;
            }
        }
        None
    }

    fn passes_mooneye_test_rom(&mut self) -> bool {
        let mut ld_b_b = 0;
        let mut frames = 0u32;

        loop {
            ld_b_b += self.mooneye_step();
            frames += 1;

            if ld_b_b > 1 {
                return self.cpu.read_reg8(Reg8::B) == 3;
            }

            if frames > 500 {
                return false;
            }
        }
    }

    fn mooneye_step(&mut self) -> u8 {
        let mut clocks_this_frame = 0u32;
        let mut ld_b_b: u8 = 0;

        loop {
            let (_line, opcode, t) = self.cpu_step();

            if opcode == 0x40 {
                ld_b_b += 1;
            }

            clocks_this_frame += u32::from(t);

            if clocks_this_frame >= CLOCKS_IN_A_FRAME {
                break;
            }
        }

        ld_b_b
    }
}
