#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn daa() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/instr/daa.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
