use std::io::Read;
use std::fs::File;

pub fn load_boot_rom() -> [u8; 0x0100] {
    let mut boot_rom: [u8; 0x0100] = [0; 0x0100];

    match File::open("roms/DMG_ROM.bin") {
        Ok(mut file) => {
            match file.read_exact(&mut boot_rom[..]) {
                Ok(_) => { return boot_rom }
                Err(_) => { panic!("couldnt read the boot rom into the buffer!") }
            };
        }
        Err(_) => { panic!("couldnt open the boot rom file") }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    /// test that the bot rom file is succesfully found and loaded
    #[test]
    fn test_boot_rom_loading() {
        let boot_rom: [u8; 0x0100] = load_boot_rom();

        assert_ne!(boot_rom[0], 0);
        assert_ne!(boot_rom[255], 0);
    }
}