//! Game Genie (ROM patch) and GameShark (RAM force) cheat support.
//!
//! Code formats and the exact decode bit-math are taken from the gambatte-libretro source:
//! Game Genie  — `libgambatte/src/mem/cartridge.cpp`  (`applyGameGenie`)
//! GameShark   — `libgambatte/src/interrupter.cpp`    (`setGameShark`)

use crate::utils::word;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A cheat code string could not be parsed (bad length, non-hex characters, or unsupported type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatError;

impl fmt::Display for CheatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid cheat code")
    }
}

impl std::error::Error for CheatError {}

/// A Game Genie code patches a ROM read at `addr` with `value`. If `compare` is set, the
/// substitution only happens when the byte currently at that ROM address equals `compare`
/// (this lets a single code target the right bank).
#[derive(Serialize, Deserialize)]
struct GameGenieCode {
    addr: u16,
    value: u8,
    compare: Option<u8>,
}

/// A GameShark code forces `value` into RAM at `addr` once per frame.
#[derive(Serialize, Deserialize)]
pub struct GameSharkCode {
    pub addr: u16,
    pub value: u8,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Cheats {
    genie: Vec<GameGenieCode>,
    shark: Vec<GameSharkCode>,
}

/// Convert a single ASCII hex digit to its nibble value.
fn hex_nibble(c: char) -> Result<u8, CheatError> {
    c.to_digit(16).map(|d| d as u8).ok_or(CheatError)
}

/// Strip dashes/whitespace and parse the remaining characters as hex nibbles.
fn clean_nibbles(code: &str) -> Result<Vec<u8>, CheatError> {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(hex_nibble)
        .collect()
}

impl Cheats {
    pub fn clear(&mut self) {
        self.genie.clear();
        self.shark.clear();
    }

    /// Parse and store a single cheat code. Classifies by cleaned hex length:
    /// 8 nibbles → GameShark, 6 or 9 nibbles → Game Genie. Returns `Err` on malformed
    /// input — never panics.
    pub fn add(&mut self, code: &str) -> Result<(), CheatError> {
        let n = clean_nibbles(code)?;
        match n.len() {
            8 => {
                self.shark.push(decode_gameshark(&n)?);
                Ok(())
            }
            6 | 9 => {
                self.genie.push(decode_game_genie(&n)?);
                Ok(())
            }
            _ => Err(CheatError),
        }
    }

    /// Apply Game Genie ROM patches to a byte read at `addr`. Hot path — fast-returns when no
    /// Game Genie codes are active.
    pub fn apply_rom(&self, addr: u16, value: u8) -> u8 {
        if self.genie.is_empty() {
            return value;
        }
        for code in &self.genie {
            if code.addr == addr && code.compare.is_none_or(|c| c == value) {
                return code.value;
            }
        }
        value
    }

    pub fn shark(&self) -> &[GameSharkCode] {
        &self.shark
    }
}

/// GameShark format `ttvvaaaa` (8 nibbles). Only type 0x01 (constant RAM write) is honored at
/// apply-time; the address is little-endian (low byte = nibbles 4-5, high byte = nibbles 6-7).
fn decode_gameshark(n: &[u8]) -> Result<GameSharkCode, CheatError> {
    if n.len() != 8 {
        return Err(CheatError);
    }
    let typ = (n[0] << 4) | n[1];
    // Only type 0x01 is supported; banked variants (0x80/0x90/...) are rejected.
    if typ != 0x01 {
        return Err(CheatError);
    }
    let value = (n[2] << 4) | n[3];
    let addr = word((n[6] << 4) | n[7], (n[4] << 4) | n[5]);
    Ok(GameSharkCode { addr, value })
}

/// Game Genie format: 6 nibbles (no compare) or 9 nibbles (with compare).
fn decode_game_genie(n: &[u8]) -> Result<GameGenieCode, CheatError> {
    if n.len() != 6 && n.len() != 9 {
        return Err(CheatError);
    }
    let value = (n[0] << 4) | n[1];
    let addr = word(((n[5] ^ 0xF) << 4) | n[2], (n[3] << 4) | n[4]) & 0x7FFF;
    let compare = if n.len() == 9 {
        let raw = ((n[6] << 4) | n[8]) ^ 0xFF; // note: n[7] is intentionally unused
        Some(raw.rotate_right(2) ^ 0x45)
    } else {
        None
    };
    Ok(GameGenieCode {
        addr,
        value,
        compare,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameshark_decode() {
        let n = clean_nibbles("011556C0").unwrap();
        let gs = decode_gameshark(&n).unwrap();
        assert_eq!(gs.addr, 0xC056);
        assert_eq!(gs.value, 0x15);
    }

    #[test]
    fn gameshark_rejects_unsupported_type() {
        assert!(decode_gameshark(&clean_nibbles("801556C0").unwrap()).is_err());
    }

    #[test]
    fn game_genie_with_compare() {
        let gg = decode_game_genie(&clean_nibbles("00A-23B-C14").unwrap()).unwrap();
        assert_eq!(gg.value, 0x00);
        assert_eq!(gg.addr, 0x4A23);
        assert_eq!(gg.compare, Some(0x8B));
    }

    #[test]
    fn game_genie_without_compare() {
        let gg = decode_game_genie(&clean_nibbles("00A-23B").unwrap()).unwrap();
        assert_eq!(gg.value, 0x00);
        assert_eq!(gg.addr, 0x4A23);
        assert_eq!(gg.compare, None);
    }

    #[test]
    fn apply_rom_substitutes() {
        let mut c = Cheats::default();
        c.add("00A-23B").unwrap(); // value 0x00 @ 0x4A23, no compare
        assert_eq!(c.apply_rom(0x4A23, 0xFF), 0x00);
        assert_eq!(c.apply_rom(0x4A24, 0xFF), 0xFF); // untouched address
    }

    #[test]
    fn apply_rom_respects_compare() {
        let mut c = Cheats::default();
        c.add("00A-23B-C14").unwrap(); // value 0x00 @ 0x4A23, compare 0x8B
        assert_eq!(c.apply_rom(0x4A23, 0x8B), 0x00); // matches compare → substitute
        assert_eq!(c.apply_rom(0x4A23, 0x42), 0x42); // wrong compare → untouched
    }

    #[test]
    fn apply_rom_empty_fast_path() {
        let c = Cheats::default();
        assert_eq!(c.apply_rom(0x4A23, 0x77), 0x77);
    }

    #[test]
    fn add_classifies_and_rejects() {
        let mut c = Cheats::default();
        assert!(c.add("011556C0").is_ok()); // gameshark
        assert!(c.add("00A-23B").is_ok()); // gg 6
        assert!(c.add("00A-23B-C14").is_ok()); // gg 9
        assert!(c.add("12345").is_err()); // bad length
        assert!(c.add("ZZZZZZ").is_err()); // bad hex
    }
}
