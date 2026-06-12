#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    boot_sclk_align_dmgabcmgb,
    "tests/mooneye/roms/serial/boot_sclk_align-dmgABCmgb.gb",
    passes_mooneye_test_rom
);
