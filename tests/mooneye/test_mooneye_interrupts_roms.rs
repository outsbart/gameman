extern crate gameman;
use gameman::gameboy::Gameboy;

#[test]
fn ie_push() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/interrupts/ie_push.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
