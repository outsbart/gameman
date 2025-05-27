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
