extern crate gameman;

#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn bits_bank1() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/bits_bank1.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn bits_bank2() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/bits_bank2.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn bits_mode() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/bits_mode.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn bits_ramg() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/bits_ramg.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn multicart_rom_8mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/multicart_rom_8Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ram_64kb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/ram_64kb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ram_256kb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/ram_256kb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_512kb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_512kb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_1mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_1Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_2mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_2Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_4mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_4Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_8mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_8Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_16mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc1/rom_16Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
