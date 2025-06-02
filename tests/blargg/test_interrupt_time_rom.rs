#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

#[test]
fn interrupt_time() {
    let mut emulator = Gameboy::new("tests/blargg/roms/interrupt_time/interrupt_time.gb");
    assert!(emulator.passes_blargg_ram_test_rom());
}
