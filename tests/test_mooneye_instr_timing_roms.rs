extern crate gameman;

use gameman::gameboy::Gameboy;

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

#[test]
fn di_timing() {
    let mut emulator = Gameboy::new("tests/di_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime0_ei() {
    let mut emulator = Gameboy::new("tests/halt_ime0_ei.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime0_nointr_timing() {
    let mut emulator = Gameboy::new("tests/halt_ime0_nointr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime1_timing() {
    let mut emulator = Gameboy::new("tests/halt_ime1_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime1_timing2() {
    let mut emulator = Gameboy::new("tests/halt_ime1_timing2-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn if_ie_registers() {
    let mut emulator = Gameboy::new("tests/if_ie_registers.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ei_sequence() {
    let mut emulator = Gameboy::new("tests/ei_sequence.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rapid_di_ei() {
    let mut emulator = Gameboy::new("tests/rapid_di_ei.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_timing() {
    let mut emulator = Gameboy::new("tests/intr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reti_intr_timing() {
    let mut emulator = Gameboy::new("tests/reti_intr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
