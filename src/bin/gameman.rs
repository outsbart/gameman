extern crate gameman;

use gameman::emu::Emulator;

fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("no gb rom file given. Usage: cargo run <rom file>");
    let mut emulator = Emulator::new(rom_path.as_str());
    emulator.run();
}
