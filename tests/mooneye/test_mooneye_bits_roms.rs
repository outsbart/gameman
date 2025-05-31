#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn mem_oam() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/bits/mem_oam.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reg_f() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/bits/reg_f.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
fn unused_hwio_gs() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/bits/unused_hwio-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
