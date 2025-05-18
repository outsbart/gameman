extern crate gameman;
use gameman::gameboy::Gameboy;

#[test]
#[ignore = "CGB-only test (measures interrupt timing at CGB double-speed); hangs on DMG emulator"]
fn interrupt_time() {
    let mut emulator = Gameboy::new("tests/blargg/roms/interrupt_time/interrupt_time.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}
