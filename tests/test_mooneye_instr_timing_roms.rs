extern crate gameman;

use gameman::gameboy::Gameboy;

// These timing tests use the DIV register approach (no OAM DMA bus blocking needed).

#[test]
fn pop_timing() {
    let mut emulator = Gameboy::new("tests/pop_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn div_timing() {
    let mut emulator = Gameboy::new("tests/div_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ei_timing() {
    let mut emulator = Gameboy::new("tests/ei_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
