#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(intr_2_0_timing, "tests/mooneye/roms/ppu/intr_2_0_timing.gb", passes_mooneye_test_rom);
test_rom!(intr_2_mode0_timing, "tests/mooneye/roms/ppu/intr_2_mode0_timing.gb", passes_mooneye_test_rom);
test_rom!(intr_2_mode0_timing_sprites, "tests/mooneye/roms/ppu/intr_2_mode0_timing_sprites.gb", passes_mooneye_test_rom);
test_rom!(intr_2_mode3_timing, "tests/mooneye/roms/ppu/intr_2_mode3_timing.gb", passes_mooneye_test_rom);
test_rom!(intr_2_oam_ok_timing, "tests/mooneye/roms/ppu/intr_2_oam_ok_timing.gb", passes_mooneye_test_rom);
test_rom!(stat_irq_blocking, "tests/mooneye/roms/ppu/stat_irq_blocking.gb", passes_mooneye_test_rom);
test_rom!(stat_lyc_onoff, "tests/mooneye/roms/ppu/stat_lyc_onoff.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] hblank_ly_scx_timing_gs, "tests/mooneye/roms/ppu/hblank_ly_scx_timing-GS.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] intr_1_2_timing_gs, "tests/mooneye/roms/ppu/intr_1_2_timing-GS.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] lcdon_timing_gs, "tests/mooneye/roms/ppu/lcdon_timing-GS.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] lcdon_write_timing_gs, "tests/mooneye/roms/ppu/lcdon_write_timing-GS.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"] vblank_stat_intr_gs, "tests/mooneye/roms/ppu/vblank_stat_intr-GS.gb", passes_mooneye_test_rom);
