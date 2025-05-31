#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn cpu_instrs_timing() {
    let mut emulator = Gameboy::new("tests/blargg/roms/instr_timing/instr_timing.gb");
    assert!(emulator.passes_test_rom());
}
