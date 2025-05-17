extern crate gameman;
use gameman::gameboy::Gameboy;

#[test]
fn basic() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma/basic.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reg_read() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma/reg_read.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn sources_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma/sources-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
