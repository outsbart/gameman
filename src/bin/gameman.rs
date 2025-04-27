extern crate gameman;

use gameman::gameboy::Gameboy;

fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("no gb rom file given. Usage: cargo run <rom file>");
    let mut gameboy = Gameboy::new(rom_path.as_str());
    gameboy.run();
}
