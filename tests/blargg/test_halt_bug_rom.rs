#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    #[ignore = "outputs to LCD only — no serial/RAM output; passes visually on our emulator"]
    halt_bug,
    "tests/blargg/roms/halt_bug.gb",
    passes_test_rom
);
