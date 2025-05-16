extern crate gameman;
use gameman::gameboy::Gameboy;

#[test]
fn oam_bug_1_lcd_sync() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/1-lcd_sync.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_2_causes() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/2-causes.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_3_non_causes() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/3-non_causes.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_4_scanline_timing() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/4-scanline_timing.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_5_timing_bug() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/5-timing_bug.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_6_timing_no_bug() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/6-timing_no_bug.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_7_timing_effect() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/7-timing_effect.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn oam_bug_8_instr_effect() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/rom_singles/8-instr_effect.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
#[ignore = "redundant with individual sub-tests which run faster in parallel"]
fn oam_bug_combined() {
    let mut emulator = Gameboy::new("tests/blargg/roms/oam_bug/oam_bug.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}
