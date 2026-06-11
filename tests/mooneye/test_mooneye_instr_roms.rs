#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(daa, "tests/mooneye/roms/instr/daa.gb", passes_mooneye_test_rom);
