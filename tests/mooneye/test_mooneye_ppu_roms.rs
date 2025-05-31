#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn intr_2_0_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_2_0_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_2_mode0_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_2_mode0_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_2_mode0_timing_sprites() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_2_mode0_timing_sprites.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_2_mode3_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_2_mode3_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_2_oam_ok_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_2_oam_ok_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn stat_irq_blocking() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/stat_irq_blocking.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn stat_lyc_onoff() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/stat_lyc_onoff.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn hblank_ly_scx_timing_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/hblank_ly_scx_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn intr_1_2_timing_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/intr_1_2_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn lcdon_timing_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/lcdon_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn lcdon_write_timing_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/lcdon_write_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn vblank_stat_intr_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ppu/vblank_stat_intr-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
