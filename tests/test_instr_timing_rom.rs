extern crate log;
extern crate gameman;


use gameman::emu::Emulator;

#[test]
fn cpu_instrs_timing() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/instr_timing.gb");
    assert!(emulator.passes_test_rom());
}
