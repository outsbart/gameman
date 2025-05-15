extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn boot_regs_dmgABC() {
    let mut emulator = Gameboy::new("tests/boot_regs-dmgABC.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn boot_div_dmgABCmgb() {
    let mut emulator = Gameboy::new("tests/boot_div-dmgABCmgb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn boot_hwio_dmgABCmgb() {
    let mut emulator = Gameboy::new("tests/boot_hwio-dmgABCmgb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets DMG rev 0, not DMG ABC"]
#[test]
fn boot_regs_dmg0() {
    let mut emulator = Gameboy::new("tests/boot_regs-dmg0.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets MGB (Pocket), not DMG ABC"]
#[test]
fn boot_regs_mgb() {
    let mut emulator = Gameboy::new("tests/boot_regs-mgb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets SGB, not DMG ABC"]
#[test]
fn boot_regs_sgb() {
    let mut emulator = Gameboy::new("tests/boot_regs-sgb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets SGB2, not DMG ABC"]
#[test]
fn boot_regs_sgb2() {
    let mut emulator = Gameboy::new("tests/boot_regs-sgb2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets DMG rev 0, not DMG ABC"]
#[test]
fn boot_div_dmg0() {
    let mut emulator = Gameboy::new("tests/boot_div-dmg0.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets Super variants, not DMG ABC"]
#[test]
fn boot_div_S() {
    let mut emulator = Gameboy::new("tests/boot_div-S.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets Super variants, not DMG ABC"]
#[test]
fn boot_div2_S() {
    let mut emulator = Gameboy::new("tests/boot_div2-S.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets DMG rev 0, not DMG ABC"]
#[test]
fn boot_hwio_dmg0() {
    let mut emulator = Gameboy::new("tests/boot_hwio-dmg0.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[ignore = "targets Super variants, not DMG ABC"]
#[test]
fn boot_hwio_S() {
    let mut emulator = Gameboy::new("tests/boot_hwio-S.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
