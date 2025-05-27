extern crate gameman;

#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn rom_512kb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_512kb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_1mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_1Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_2mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_2Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_4mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_4Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_8mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_8Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_16mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_16Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_32mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_32Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_64mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc5/rom_64Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
