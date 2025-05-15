extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn div_write() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/div_write.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim00() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim00.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim01() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim01.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim10() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim10.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim11() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim11.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim00_div_trigger() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim00_div_trigger.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim01_div_trigger() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim01_div_trigger.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim10_div_trigger() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim10_div_trigger.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tim11_div_trigger() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tim11_div_trigger.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tima_reload() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tima_reload.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tma_write_reloading() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tma_write_reloading.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn tima_write_reloading() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/tima_write_reloading.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rapid_toggle() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/timer/rapid_toggle.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
