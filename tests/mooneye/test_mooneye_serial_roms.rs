extern crate gameman;
use gameman::gameboy::Gameboy;

#[test]
fn boot_sclk_align_dmgabcmgb() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/serial/boot_sclk_align-dmgABCmgb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
