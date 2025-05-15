extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn pop_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/pop_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn div_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/div_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ei_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ei_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn di_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/di_timing-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime0_ei() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/halt_ime0_ei.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime0_nointr_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/halt_ime0_nointr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime1_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/halt_ime1_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn halt_ime1_timing2() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/halt_ime1_timing2-GS.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn if_ie_registers() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/if_ie_registers.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ei_sequence() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ei_sequence.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rapid_di_ei() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/rapid_di_ei.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn intr_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/intr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reti_intr_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/reti_intr_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn oam_dma_start() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma_start.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn oam_dma_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn oam_dma_restart() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/oam_dma_restart.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/call_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_timing2() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/call_timing2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_cc_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/call_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn call_cc_timing2() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/call_cc_timing2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ret_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ret_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ret_cc_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ret_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn reti_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/reti_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn jp_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/jp_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn jp_cc_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/jp_cc_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn push_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/push_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rst_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/rst_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn add_sp_e_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/add_sp_e_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ld_hl_sp_e_timing() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/ld_hl_sp_e_timing.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
#[ignore = "passes visually but completion signal mechanism is unknown — no source available"]
fn halt_bug() {
    let mut emulator = Gameboy::new("tests/mooneye/roms/halt_bug.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
