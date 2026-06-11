#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(interrupt_time, "tests/blargg/roms/interrupt_time/interrupt_time.gb", passes_blargg_ram_test_rom);
