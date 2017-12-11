extern crate gameman;
extern crate env_logger;

use gameman::emu::Emulator;

fn main() {
    env_logger::init();
    let mut emulator = Emulator::new();
    emulator.load_bios();
    emulator.load_rom("roms/Tetris (World).gb");
//    emulator.load_rom("roms/individual/03-op sp,hl.gb");
    emulator.run();
}
