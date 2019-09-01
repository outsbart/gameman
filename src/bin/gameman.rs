extern crate gameman;

use gameman::emu::Emulator;

fn main() {
    let mut emulator = Emulator::new("roms/Tetris.gb");
    emulator.run();
}
