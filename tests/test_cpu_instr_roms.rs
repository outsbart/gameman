extern crate gameman;


use gameman::emu::Emulator;

#[test]
fn cpu_instrs_01() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/01-special.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_02() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/02-interrupts.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_03() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/03-op sp,hl.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_04() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/04-op r,imm.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_05() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/05-op rp.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_06() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/06-ld r,r.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_07() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/07-jr,jp,call,ret,rst.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_08() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/08-misc instrs.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_09() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/09-op r,r.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_10() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/10-bit ops.gb");
    assert!(emulator.passes_test_rom());
}

#[test]
fn cpu_instrs_11() {
    let mut emulator = Emulator::new();
    emulator.load_rom("tests/cpu_instrs/11-op a,(hl).gb");
    assert!(emulator.passes_test_rom());
}

