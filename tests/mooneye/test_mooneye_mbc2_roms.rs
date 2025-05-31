#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn bits_ramg() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/bits_ramg.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn bits_romb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/bits_romb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn bits_unused() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/bits_unused.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn ram() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/ram.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_512kb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/rom_512kb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_1mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/rom_1Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}

#[test]
fn rom_2mb() {
    let mut emulator = Gameboy::new_clean("tests/mooneye/roms/mbc2/rom_2Mb.gb");
    assert!(emulator.passes_mooneye_test_rom());
}
