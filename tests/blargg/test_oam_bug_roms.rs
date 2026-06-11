#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(oam_bug_1_lcd_sync, "tests/blargg/roms/oam_bug/rom_singles/1-lcd_sync.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_2_causes, "tests/blargg/roms/oam_bug/rom_singles/2-causes.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_3_non_causes, "tests/blargg/roms/oam_bug/rom_singles/3-non_causes.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_4_scanline_timing, "tests/blargg/roms/oam_bug/rom_singles/4-scanline_timing.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_5_timing_bug, "tests/blargg/roms/oam_bug/rom_singles/5-timing_bug.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_6_timing_no_bug, "tests/blargg/roms/oam_bug/rom_singles/6-timing_no_bug.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_7_timing_effect, "tests/blargg/roms/oam_bug/rom_singles/7-timing_effect.gb", passes_blargg_ram_test_rom);
test_rom!(oam_bug_8_instr_effect, "tests/blargg/roms/oam_bug/rom_singles/8-instr_effect.gb", passes_blargg_ram_test_rom);
test_rom!(
    #[ignore = "redundant with individual sub-tests which run faster in parallel"]
    oam_bug_combined,
    "tests/blargg/roms/oam_bug/oam_bug.gb",
    passes_blargg_ram_test_rom
);
