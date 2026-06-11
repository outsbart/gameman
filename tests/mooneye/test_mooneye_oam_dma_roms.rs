#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(basic, "tests/mooneye/roms/oam_dma/basic.gb", passes_mooneye_test_rom);
test_rom!(reg_read, "tests/mooneye/roms/oam_dma/reg_read.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] sources_gs, "tests/mooneye/roms/oam_dma/sources-GS.gb", passes_mooneye_test_rom);
