#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(ie_push, "tests/mooneye/roms/interrupts/ie_push.gb", passes_mooneye_test_rom);
