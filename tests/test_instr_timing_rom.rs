extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn cpu_instrs_timing() {
    let mut emulator = Gameboy::new("tests/instr_timing.gb");
    assert!(emulator.passes_test_rom());
}
