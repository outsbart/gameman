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

#[test]
fn oam_dma_start() {
    let mut emulator = Gameboy::new("tests/oam_dma_start.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn oam_dma_timing() {
    let mut emulator = Gameboy::new("tests/oam_dma_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn oam_dma_restart() {
    let mut emulator = Gameboy::new("tests/oam_dma_restart.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_timing() {
    let mut emulator = Gameboy::new("tests/call_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_timing2() {
    let mut emulator = Gameboy::new("tests/call_timing2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_cc_timing() {
    let mut emulator = Gameboy::new("tests/call_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_cc_timing2() {
    let mut emulator = Gameboy::new("tests/call_cc_timing2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ret_timing() {
    let mut emulator = Gameboy::new("tests/ret_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ret_cc_timing() {
    let mut emulator = Gameboy::new("tests/ret_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reti_timing() {
    let mut emulator = Gameboy::new("tests/reti_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn jp_timing() {
    let mut emulator = Gameboy::new("tests/jp_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn jp_cc_timing() {
    let mut emulator = Gameboy::new("tests/jp_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn push_timing() {
    let mut emulator = Gameboy::new("tests/push_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rst_timing() {
    let mut emulator = Gameboy::new("tests/rst_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn add_sp_e_timing() {
    let mut emulator = Gameboy::new("tests/add_sp_e_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ld_hl_sp_e_timing() {
    let mut emulator = Gameboy::new("tests/ld_hl_sp_e_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
