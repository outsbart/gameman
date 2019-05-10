extern crate env_logger;
extern crate gameman;

use gameman::emu::Emulator;

fn main() {
    env_logger::init();
    let mut emulator = Emulator::new();
    emulator.load_bios();
    emulator.load_rom("roms/Tetris (World) (Rev A).gb");
    emulator.run();
}
