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

#[test]
fn tim01() {
    let mut emulator = Gameboy::new("tests/timer/tim01.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim10() {
    let mut emulator = Gameboy::new("tests/timer/tim10.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim11() {
    let mut emulator = Gameboy::new("tests/timer/tim11.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
