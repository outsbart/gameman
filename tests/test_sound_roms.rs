extern crate gameman;


use gameman::emu::Emulator;

#[test]
fn sound_registers() {
    let mut emulator = Emulator::new("tests/sound/01-registers.gb");
    assert!(emulator.passes_test_rom());
}
