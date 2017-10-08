extern crate gameman;
use gameman::utils::load_boot_rom;


fn main() {
    let i: u64 = u32::max_value() as u64 + u32::max_value() as u64;

    println!("{}", u64::max_value()-i);
}
