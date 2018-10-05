use std::io::Read;
use std::fs::File;
use std::mem;


pub fn load_rom(path: &str) -> [u8; 0x8000] {
    let mut boot_rom: [u8; 0x8000] = [0; 0x8000];

    match File::open(path) {
        Ok(mut file) => {
            match file.read_exact(&mut boot_rom[..]) {
                Ok(_) => { return boot_rom }
                Err(_) => { panic!("couldnt read the rom into the buffer!") }
            };
        }
        Err(_) => { panic!("couldnt open the rom file") }
    }
}

pub fn load_boot_rom() -> [u8; 0x0100] { // TODO: make a generic function for loading roms
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

pub fn rotate_left(unsigned: u8) -> u16 {
    u16::from(unsigned << 1)
}

pub fn rotate_right(unsigned: u8) -> u16 {
    u16::from(unsigned >> 1)
}

pub fn swap_nibbles(unsigned: u8) -> u16 {
    // swap the nibbles
    let first_nibble = (unsigned & 0xF0) >> 4;
    let second_nibble = (unsigned & 0x0F) << 4;
    (first_nibble + second_nibble) as u16
}

pub fn parse_hex(number: &str) -> u16 {
    u16::from_str_radix(number, 16).expect(format!("cant read {} yet!!!", number).as_ref())
}

pub fn reset_bit(position: u8, number: u8) -> u16 {
    (!(1u16<<position) & number as u16) as u16
}

pub fn set_bit(position: u8, number: u8) -> u16 {
    ((1u16<<position) | number as u16) as u16
}

pub fn add_words(a: u16, b: u16, c: u16) -> (u16, bool, bool) {
    let a = a as u32;
    let b = b as u32;

    let res = a.wrapping_add(b).wrapping_add(c as u32);
    let carry = res & 0x10000 != 0;
    let halfcarry = (a ^ b ^ res) & 0x1000 != 0;

    (res as u16, carry, halfcarry)
}

pub fn add_word_with_signed(a: u16, b: u16, _:u16) -> (u16, bool, bool) {
    let a = a as i32;
    let b = b as u8 as i8 as i32;
    let res = a.wrapping_add(b);

    let carry = (a ^ b ^ res) & 0x100 != 0;
    let halfcarry = (a ^ b ^ res) & 0x10 != 0;

    (res as u32 as u16, carry, halfcarry)
}

pub fn add_bytes(a: u16, b: u16, c: u16) -> (u16, bool, bool) {
    let res = a.wrapping_add(b).wrapping_add(c);
    let carry = res & 0x100 != 0;
    let halfcarry = (a ^ b ^ res) & 0x10 != 0;

    (res, carry, halfcarry)
}

pub fn sub_bytes(a: u16, b: u16, c: u16) -> (u16, bool, bool) {
    let a = a as u32;
    let b = b as u32;

    let res = a.wrapping_sub(b).wrapping_sub(c as u32);
    let carry = res & 0x100 != 0;
    let halfcarry = (a ^ b ^ res) & 0x10 != 0;

    (res as u16, carry, halfcarry)
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

    #[test]
    fn test_rotate_left() {
        // it doesnt really rotate... it's a shift and adds the Carry
        assert_eq!(rotate_left(0b00000001u8), 0b0000000000000010u16);
        assert_eq!(rotate_left(0b10000000u8), 0b0000000000000000u16);
    }

    #[test]
    fn test_swap() {
        assert_eq!(swap_nibbles(0xF0u8), 0x000Fu16);
        assert_eq!(swap_nibbles(0x0Fu8), 0x00F0u16);
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("20"), 0x0020u16);
    }

    #[test]
    fn test_reset_bit() {
        assert_eq!(reset_bit(0, 0b1111_1110), 0b0000_0000_1111_1110);
        assert_eq!(reset_bit(0, 0b1111_1111), 0b0000_0000_1111_1110);
        assert_eq!(reset_bit(1, 0b1111_1111), 0b0000_0000_1111_1101);
        assert_eq!(reset_bit(2, 0b1111_1111), 0b0000_0000_1111_1011);
        assert_eq!(reset_bit(3, 0b1111_1111), 0b0000_0000_1111_0111);
        assert_eq!(reset_bit(4, 0b1111_1111), 0b0000_0000_1110_1111);
        assert_eq!(reset_bit(5, 0b1111_1111), 0b0000_0000_1101_1111);
        assert_eq!(reset_bit(6, 0b1111_1111), 0b0000_0000_1011_1111);
        assert_eq!(reset_bit(7, 0b1111_1111), 0b0000_0000_0111_1111);

    }

    #[test]
    fn test_rust_shift() {
        assert_eq!(u8::from(true), 0x1);
        assert_eq!(u8::from(true) << 1, 0x2);
    }
}