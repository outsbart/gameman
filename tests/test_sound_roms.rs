extern crate gameman;

use gameman::gameboy::Gameboy;

#[test]
fn sound_01_registers() {
    let mut emulator = Gameboy::new("tests/sound/01-registers.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_02_len_ctr() {
    let mut emulator = Gameboy::new("tests/sound/02-len ctr.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_03_trigger() {
    let mut emulator = Gameboy::new("tests/sound/03-trigger.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_04_sweep() {
    let mut emulator = Gameboy::new("tests/sound/04-sweep.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_05_sweep_details() {
    let mut emulator = Gameboy::new("tests/sound/05-sweep details.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_06_overflow_on_trigger() {
    let mut emulator = Gameboy::new("tests/sound/06-overflow on trigger.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_07_len_sweep_period_sync() {
    let mut emulator = Gameboy::new("tests/sound/07-len sweep period sync.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_08_len_ctr_during_power() {
    let mut emulator = Gameboy::new("tests/sound/08-len ctr during power.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_09_wave_read_while_on() {
    let mut emulator = Gameboy::new("tests/sound/09-wave read while on.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_10_wave_trigger_while_on() {
    let mut emulator = Gameboy::new("tests/sound/10-wave trigger while on.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_11_regs_after_power() {
    let mut emulator = Gameboy::new("tests/sound/11-regs after power.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[test]
fn sound_12_wave_write_while_on() {
    let mut emulator = Gameboy::new("tests/sound/12-wave write while on.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}

#[ignore = "redundant with individual sub-tests which run faster in parallel"]
#[test]
fn sound_combined() {
    let mut emulator = Gameboy::new("tests/sound/dmg_sound.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}
