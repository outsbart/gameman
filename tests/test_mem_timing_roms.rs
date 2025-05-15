extern crate gameman;

use gameman::gameboy::Gameboy;

#[ignore = "redundant: tests identical logic to mem_timing_2 sub-tests, which run faster in parallel"]
#[test]
fn mem_timing_combined() {
    let mut emulator = Gameboy::new("tests/mem_timing/mem_timing.gb");
    assert!(emulator.passes_test_rom());
}

#[ignore = "redundant with individual sub-tests which run faster in parallel"]
#[test]
fn mem_timing_2_combined() {
    let mut emulator = Gameboy::new("tests/mem_timing-2/mem_timing.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn mem_timing_2_01_read_timing() {
    let mut emulator = Gameboy::new("tests/mem_timing-2/01-read_timing.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn mem_timing_2_02_write_timing() {
    let mut emulator = Gameboy::new("tests/mem_timing-2/02-write_timing.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn mem_timing_2_03_modify_timing() {
    let mut emulator = Gameboy::new("tests/mem_timing-2/03-modify_timing.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}
