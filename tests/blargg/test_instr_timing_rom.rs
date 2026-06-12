#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    cpu_instrs_timing,
    "tests/blargg/roms/instr_timing/instr_timing.gb",
    passes_test_rom
);
