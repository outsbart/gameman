extern crate gameman;

use gameman::emu::Emulator;

fn main() {
    let mut emulator = Emulator::new();
    emulator.load_rom("roms/Tetris (World).gb");
    emulator.run();
}
