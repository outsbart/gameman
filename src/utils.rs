use std::io::Read;
use std::fs::File;
use std::mem;

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

pub fn u16_to_i16(unsigned: u16) -> i16 {
    unsafe {
        mem::transmute::<u16, i16>(unsigned)
    }
}

pub fn u8_to_i8(unsigned: u8) -> i8 {
    unsafe {
        mem::transmute::<u8, i8>(unsigned)
    }
}


#[allow(overflowing_literals)]
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

    #[test]
    fn test_u8_to_i8() {
        assert_eq!(u8_to_i8(0b0111_1111u8), 0b0111_1111i8);
        assert_eq!(u8_to_i8(0b1111_1111u8), 0b1111_1111i8);
        assert_eq!(u8_to_i8(0b0000_1111u8), 0b0000_1111i8);
        assert_eq!(u8_to_i8(0b1111_1110u8), 0b1111_1110i8);
    }
}