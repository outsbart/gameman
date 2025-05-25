extern crate gameman;

#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
#[ignore = "outputs to LCD only — no serial/RAM output; passes visually on our emulator"]
fn halt_bug() {
    let mut emulator = Gameboy::new("tests/blargg/roms/halt_bug.gb");
    assert!(emulator.passes_test_rom());
}
