pub fn swap_nibbles(byte: u8) -> u8 {
    byte.rotate_left(4)
}

pub fn reset_bit(byte: u8, n: u8) -> u8 {
    byte & !(1u8 << n)
}

pub fn set_bit(byte: u8, n: u8) -> u8 {
    byte | (1u8 << n)
}

/// True if bit `n` (0-7) of `byte` is set.
#[inline]
pub fn bit(byte: u8, n: u8) -> bool {
    byte & (1 << n) != 0
}

/// `1 << n` if `cond`, else 0. For packing boolean flags into register reads.
#[inline]
pub fn bit_if(cond: bool, n: u8) -> u8 {
    u8::from(cond) << n
}

/// Extract a `width`-bit field starting at bit `shift`.
#[inline]
pub fn field(byte: u8, shift: u8, width: u8) -> u8 {
    (byte >> shift) & (0xFF >> (8 - width))
}

/// Assemble a u16 from a high byte and a low byte: `(hi << 8) | lo`.
#[inline]
pub fn word(hi: u8, lo: u8) -> u16 {
    (u16::from(hi) << 8) | u16::from(lo)
}

/// Low byte of a word.
#[inline]
pub fn lo(w: u16) -> u8 {
    (w & 0x00FF) as u8
}

/// High byte of a word.
#[inline]
pub fn hi(w: u16) -> u8 {
    (w >> 8) as u8
}

/// Upper nibble, shifted down to 0x0–0xF.
#[inline]
pub fn hi_nibble(byte: u8) -> u8 {
    byte >> 4
}

/// Lower nibble, 0x0–0xF.
#[inline]
pub fn lo_nibble(byte: u8) -> u8 {
    byte & 0x0F
}

pub fn add_words(a: u16, b: u16, c: u16) -> (u16, bool, bool) {
    let a = u32::from(a);
    let b = u32::from(b);

    let res = a.wrapping_add(b).wrapping_add(u32::from(c));
    let carry = res & 0x10000 != 0;
    let halfcarry = (a ^ b ^ res) & 0x1000 != 0;

    (res as u16, carry, halfcarry)
}

pub fn add_word_with_signed(a: u16, b: u16, _: u16) -> (u16, bool, bool) {
    let a = i32::from(a);
    let b = i32::from(b as u8 as i8);
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
    let a = u32::from(a);
    let b = u32::from(b);

    let res = a.wrapping_sub(b).wrapping_sub(u32::from(c));
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
        assert_eq!(swap_nibbles(0xF0u8), 0x0Fu8);
        assert_eq!(swap_nibbles(0x0Fu8), 0xF0u8);
    }

    #[test]
    fn test_reset_bit() {
        assert_eq!(reset_bit(0b1111_1110, 0), 0b1111_1110u8);
        assert_eq!(reset_bit(0b1111_1111, 0), 0b1111_1110u8);
        assert_eq!(reset_bit(0b1111_1111, 1), 0b1111_1101u8);
        assert_eq!(reset_bit(0b1111_1111, 2), 0b1111_1011u8);
        assert_eq!(reset_bit(0b1111_1111, 3), 0b1111_0111u8);
        assert_eq!(reset_bit(0b1111_1111, 4), 0b1110_1111u8);
        assert_eq!(reset_bit(0b1111_1111, 5), 0b1101_1111u8);
        assert_eq!(reset_bit(0b1111_1111, 6), 0b1011_1111u8);
        assert_eq!(reset_bit(0b1111_1111, 7), 0b0111_1111u8);
    }

    #[test]
    fn test_bit() {
        assert!(bit(0b1000_0000, 7));
        assert!(!bit(0b1000_0000, 6));
        assert!(bit(0b0000_0001, 0));
        assert!(!bit(0b0000_0001, 1));
    }

    #[test]
    fn test_bit_if() {
        assert_eq!(bit_if(true, 7), 0b1000_0000);
        assert_eq!(bit_if(false, 7), 0);
        assert_eq!(bit_if(true, 0), 1);
        assert_eq!(bit_if(true, 3) | bit_if(true, 1), 0b0000_1010);
    }

    #[test]
    fn test_field() {
        assert_eq!(field(0b0110_0000, 5, 2), 0b11);
        assert_eq!(field(0b0001_1100, 2, 3), 0b111);
        assert_eq!(field(0xFF, 4, 4), 0x0F);
        assert_eq!(field(0b1010_1010, 1, 3), 0b101);
    }

    #[test]
    fn test_word() {
        assert_eq!(word(0xAB, 0xCD), 0xABCD);
        assert_eq!(word(0x00, 0xFF), 0x00FF);
        assert_eq!(word(0xFF, 0x00), 0xFF00);
    }

    #[test]
    fn test_hi_lo() {
        assert_eq!(hi(0xABCD), 0xAB);
        assert_eq!(lo(0xABCD), 0xCD);
        assert_eq!(hi(0x0000), 0x00);
        assert_eq!(lo(0xFFFF), 0xFF);
    }

    #[test]
    fn test_nibbles() {
        assert_eq!(hi_nibble(0xAB), 0x0A);
        assert_eq!(lo_nibble(0xAB), 0x0B);
        assert_eq!(hi_nibble(0xF0), 0x0F);
        assert_eq!(lo_nibble(0x0F), 0x0F);
    }
}
