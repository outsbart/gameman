extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn div_write() {
    let mut emulator = Gameboy::new("tests/timer/div_write.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim00() {
    let mut emulator = Gameboy::new("tests/timer/tim00.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
