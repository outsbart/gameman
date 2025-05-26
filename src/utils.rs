pub fn swap_nibbles(unsigned: u8) -> u16 {
    let first_nibble = (unsigned & 0xF0) >> 4;
    let second_nibble = (unsigned & 0x0F) << 4;
    (first_nibble + second_nibble) as u16
}

pub fn reset_bit(position: u8, number: u8) -> u16 {
    !(1u16 << position) & number as u16
}

pub fn set_bit(position: u8, number: u8) -> u16 {
    (1u16 << position) | number as u16
}

pub fn add_words(a: u16, b: u16, c: u16) -> (u16, bool, bool) {
    let a = a as u32;
    let b = b as u32;

    let res = a.wrapping_add(b).wrapping_add(c as u32);
    let carry = res & 0x10000 != 0;
    let halfcarry = (a ^ b ^ res) & 0x1000 != 0;

    (res as u16, carry, halfcarry)
}

pub fn add_word_with_signed(a: u16, b: u16, _: u16) -> (u16, bool, bool) {
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

    #[test]
    fn test_swap() {
        assert_eq!(swap_nibbles(0xF0u8), 0x000Fu16);
        assert_eq!(swap_nibbles(0x0Fu8), 0x00F0u16);
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
}
