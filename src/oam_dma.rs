use crate::gpu::GPUMemoriesAccess;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OamDma {
    pub source: u8,
    remaining: u8,
    startup: u8,
    mode_before: u8,
    row_before: u8,
}

impl Default for OamDma {
    fn default() -> Self {
        Self::new()
    }
}

impl OamDma {
    pub fn new() -> OamDma {
        OamDma {
            source: 0,
            remaining: 0,
            startup: 0,
            mode_before: 0,
            row_before: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0
    }

    /// Write to 0xFF46: record the source page and arm the 2-M-cycle startup delay.
    pub fn trigger(&mut self, byte: u8) {
        self.source = byte;
        self.startup = 2;
    }

    /// Advance one M-cycle. Returns Some(start_addr) exactly when the 160-byte copy
    /// should fire; sets remaining=0 so read_byte calls inside the loop are unblocked.
    pub fn tick_m(&mut self) -> Option<u16> {
        if self.startup > 0 {
            self.startup -= 1;
            if self.startup == 0 {
                self.remaining = 0;
                return Some((self.source as u16) << 8);
            }
        } else if self.remaining > 0 {
            self.remaining -= 1;
        }
        None
    }

    /// Called after the copy loop to start the 160-M-cycle bus-blocking countdown.
    pub fn finish(&mut self) {
        self.remaining = 160;
    }

    /// Snapshot GPU mode and OAM scan row before each instruction fetch.
    pub fn snapshot<G: GPUMemoriesAccess>(&mut self, gpu: &G) {
        self.mode_before = gpu.gpu_mode();
        self.row_before = gpu.oam_scan_row();
    }

    /// Apply OAM corruption for the just-executed opcode if mode-2 was active at fetch time.
    pub fn handle_corruption<G: GPUMemoriesAccess>(&self, gpu: &mut G, opcode: u8, rr: u16) {
        if self.mode_before != 2 {
            return;
        }
        let row = self.row_before;
        let in_oam_bus = |addr: u16| (addr >> 8) == 0xFE;
        match opcode {
            // INC/DEC rr: bug fires after M1 (+4T = +8 bytes in OAM scan)
            0x03 | 0x0B | 0x13 | 0x1B | 0x23 | 0x2B | 0x33 | 0x3B if in_oam_bus(rr) => {
                gpu.apply_oam_corruption(row.saturating_add(8));
            }
            // LD A,(HL±): M2 memory read → read corruption (+8 bytes)
            0x2A | 0x3A if in_oam_bus(rr) => {
                gpu.apply_oam_read_corruption(row.saturating_add(8));
            }
            // POP rr: M2 reads SP (+8 bytes), M3 reads SP+1 (+16 bytes) — read corruption
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                if in_oam_bus(rr) {
                    gpu.apply_oam_read_corruption(row.saturating_add(8));
                }
                if in_oam_bus(rr.wrapping_add(1)) {
                    gpu.apply_oam_read_corruption(row.saturating_add(16));
                }
            }
            // PUSH rr: M2 internal (+8), M3 writes SP-1 (+16), M4 writes SP-2 (+24)
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                if in_oam_bus(rr) {
                    gpu.apply_oam_corruption(row.saturating_add(8));
                }
                if in_oam_bus(rr.wrapping_sub(1)) {
                    gpu.apply_oam_corruption(row.saturating_add(16));
                }
                if in_oam_bus(rr.wrapping_sub(2)) {
                    gpu.apply_oam_corruption(row.saturating_add(24));
                }
            }
            _ => {}
        }
    }
}

// --- OAM bug corruption formulas ---
// These mutate the 160-byte OAM directly. `r` is the byte offset of the row being
// corrupted; both are no-ops when the LCD is off or the row is outside the affected range.

/// Write corruption (INC/DEC/PUSH while OAM is being scanned), GB_trigger_oam_bug in SameBoy.
pub(crate) fn apply_oam_corruption(oam: &mut [u8; 160], lcd_enabled: bool, r: u8) {
    if !lcd_enabled {
        return;
    }
    let r = r as usize;
    if !(8..=152).contains(&r) {
        return;
    }
    let a = oam[r] as u16 | ((oam[r + 1] as u16) << 8);
    let b = oam[r - 8] as u16 | ((oam[r - 7] as u16) << 8);
    let c = oam[r - 4] as u16 | ((oam[r - 3] as u16) << 8);
    let result = ((a ^ c) & (b ^ c)) ^ c;
    oam[r] = result as u8;
    oam[r + 1] = (result >> 8) as u8;
    for i in 2..8usize {
        oam[r + i] = oam[r - 8 + i];
    }
}

/// Read corruption (POP / LD A,(HL±) while OAM is being scanned), GB_trigger_oam_bug_read
/// in SameBoy. Each formula variant modifies the scan row (r-8), then the scan row is always
/// copied to the corrupted row (r) — matching the unconditional copy in SameBoy.
pub(crate) fn apply_oam_read_corruption(oam: &mut [u8; 160], lcd_enabled: bool, r: u8) {
    if !lcd_enabled {
        return;
    }
    let r = r as usize;
    if !(8..=152).contains(&r) {
        return;
    }
    match r & 0x18 {
        // Standard: b | (a & c) on scan row bytes 0-1
        0x08 | 0x18 => {
            for i in 0..2usize {
                let a = oam[r + i];
                let b = oam[r - 8 + i];
                let c = oam[r - 4 + i];
                oam[r - 8 + i] = b | (a & c);
            }
        }
        // Secondary: formula on scan row bytes 0-1; copy scan row → prev2
        0x10 => {
            for i in 0..2usize {
                let a = oam[r - 16 + i];
                let b = oam[r - 8 + i];
                let c = oam[r + i];
                let d = oam[r - 4 + i];
                oam[r - 8 + i] = (b & (a | c | d)) | (a & c & d);
            }
            for i in 0..8usize {
                oam[r - 16 + i] = oam[r - 8 + i];
            }
        }
        // Tertiary/quaternary: formula on scan row bytes 0-1; copy scan row → prev2 and prev4
        0x00 => {
            for i in 0..2usize {
                let a = oam[r + i];
                let b = oam[r - 4 + i];
                let c = oam[r - 8 + i];
                let d = oam[r - 16 + i];
                let e = oam[r - 32 + i];
                oam[r - 8 + i] = match r {
                    0x20 => (c & (a | b | d | e)) | (a & b & d & e), // tertiary_2
                    0x40 => {
                        // quaternary_dmg
                        // SameBoy: (e & (h|g|(~d&f)|c|b)) | (c&g&h)
                        // where b=oam[r+i], c=oam[r-4+i], d=oam[r-6+i], e=oam[r-8+i],
                        //       f=oam[r-14+i], g=oam[r-16+i], h=oam[r-32+i]
                        let sb_d = oam[r - 6 + i];
                        let sb_f = oam[r - 14 + i];
                        // my c=sb_e, my b=sb_c, my a=sb_b, my d=sb_g, my e=sb_h
                        (c & (e | d | ((!sb_d) & sb_f) | b | a)) | (b & d & e)
                    }
                    0x60 => (c & (a | b | d | e)) | (b & d & e), // tertiary_3
                    _ => c | (a & b & d & e),                    // tertiary_1 (r==0x80)
                };
            }
            for i in 0..8usize {
                oam[r - 16 + i] = oam[r - 8 + i];
                oam[r - 32 + i] = oam[r - 8 + i];
            }
        }
        _ => return,
    }
    // Always: copy (possibly modified) scan row → corrupted row
    for i in 0..8usize {
        oam[r + i] = oam[r - 8 + i];
    }
    if r == 0x80 {
        for i in 0..8usize {
            oam[i] = oam[0x80 + i];
        }
    }
}
