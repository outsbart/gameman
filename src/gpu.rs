use crate::cpu::is_bit_set;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

const TILES_IN_A_TILEMAP_ROW: usize = 32;
const TILES_IN_A_TILEMAP_COL: usize = 32;
const TILES_IN_A_SCREEN_ROW: usize = 20;
const TILES_IN_A_SCREEN_COL: usize = 18;
const TILE_SIZE: usize = 8;

const TILEMAP0_OFFSET: usize = 0x9800 - 0x8000;
const TILEMAP1_OFFSET: usize = 0x9C00 - 0x8000;

const TILEDATA1_OFFSET: usize = 0;
const TILEDATA0_OFFSET: usize = 0x9000 - 0x8000;
const TILEDATA_SHARED: usize = 0x8800 - 0x8000; // when tile index >= 128

/// Expose the memories of the GPU
pub trait GPUMemoriesAccess {
    fn read_oam(&mut self, addr: u16) -> u8;
    fn write_oam(&mut self, addr: u16, byte: u8);
    fn read_vram(&mut self, addr: u16) -> u8;
    fn write_vram(&mut self, addr: u16, byte: u8);
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);
    fn step(&mut self, t: u8) -> (bool, bool);
    fn post_boot_init(&mut self) {}
    fn gpu_mode(&self) -> u8 {
        0
    }
    fn oam_scan_row(&self) -> u8 {
        0
    }
    fn apply_oam_corruption(&mut self, _row: u8) {}
    fn apply_oam_read_corruption(&mut self, _row: u8) {}
    fn get_line(&self) -> u8 {
        0
    }
    fn is_lcd_enabled(&self) -> bool {
        false
    }
    fn cgb_mode(&self) -> bool {
        false
    }
    fn take_hblank(&mut self) -> bool {
        false
    }
    fn oam_accessible(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Colour {
    Off = 0,
    Light = 1,
    Dark = 2,
    On = 3,
}

impl Colour {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Colour::Light,
            2 => Colour::Dark,
            3 => Colour::On,
            _ => Colour::Off,
        }
    }
}

impl From<u8> for Colour {
    fn from(val: u8) -> Self {
        match val {
            0 => Colour::Off,
            1 => Colour::Light,
            2 => Colour::Dark,
            3 => Colour::On,
            _ => panic!("Impossible colour"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Palette {
    colour_3: Colour,
    colour_2: Colour,
    colour_1: Colour,
    colour_0: Colour,
    byte: u8,
}

impl Palette {
    fn new() -> Self {
        Palette {
            colour_3: Colour::Off,
            colour_2: Colour::Off,
            colour_1: Colour::Off,
            colour_0: Colour::Off,
            byte: 0xFF,
        }
    }

    fn get(&self, colour_number: u8) -> Colour {
        match colour_number {
            3 => self.colour_3,
            2 => self.colour_2,
            1 => self.colour_1,
            _ => self.colour_0,
        }
    }

    fn update(&mut self, value: u8) {
        self.colour_0 = Colour::from_u8(value & 0b0000_0011);
        self.colour_1 = Colour::from_u8((value & 0b0000_1100) >> 2);
        self.colour_2 = Colour::from_u8((value & 0b0011_0000) >> 4);
        self.colour_3 = Colour::from_u8((value & 0b1100_0000) >> 6);
        self.byte = value;
    }
}

#[derive(Serialize, Deserialize)]
pub struct GPU {
    #[serde(with = "BigArray")]
    pub vram: [u8; 16384],
    #[serde(with = "BigArray")]
    pub oam: [u8; 160],
    #[serde(with = "BigArray")]
    buffer: [u32; 160 * 144],

    // CGB support
    pub cgb_mode: bool,
    vram_bank: u8,
    #[serde(with = "BigArray")]
    bg_palette_data: [u8; 64],
    #[serde(with = "BigArray")]
    obj_palette_data: [u8; 64],
    bcps: u8,
    ocps: u8,
    bg_colors: [[u32; 4]; 8],
    obj_colors: [[u32; 4]; 8],
    dmg_palette: [u32; 4],

    modeclock: u16,
    mode: u8,
    line: u8,

    bg_enabled: bool,         // draw bg? (DMG only; always drawn in CGB mode)
    bg_master_priority: bool, // CGB only: LCDC bit 0; when false, sprites always win
    obj_enabled: bool,        // draw sprites?
    obj_size: bool,           // sprite is tall 16 or 8 pixel?
    bg_map: bool,             // which tilemap to use for the bg
    bg_tile: bool,            // tiles data to use for both bg and window
    window_enabled: bool,     // draw window?
    window_map: bool,         // which tilemap use for the window?
    lcd_enabled: bool,

    compare_enabled: bool,   // stat reg. Should compare with compare line?
    compare_line: u8,        // when line == compare_line an interrupt is triggered
    hblank_flag: bool,       // set on mode 3→0 transition; consumed by MMU for HBDMA
    mode0_int_enabled: bool, // stat reg. Fire STAT interrupt on mode 0 (HBlank) start?
    mode1_int_enabled: bool, // stat reg. Fire STAT interrupt on mode 1 (VBlank) start?
    mode2_int_enabled: bool, // stat reg. Fire STAT interrupt on mode 2 start?
    accessed_oam_row: u8,    // byte offset into OAM being scanned; 0xFF outside mode 2
    mode3_extra: u16,        // extra T-cycles added to mode 3 by sprites (subtracted from mode 0)
    stat_line: bool,         // current combined STAT interrupt line (for blocking logic)
    pending_stat: bool,      // STAT interrupt triggered by a register write (FF41/FF45)
    lyc_flag: bool,          // LY==LYC comparison result; frozen when LCD is disabled
    lcd_startup_ticks: u8,   // T-cycles remaining in the mode-0 window right after LCD enable

    scroll_x: u8,
    scroll_y: u8,
    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,
    window_x: u8,
    window_y: u8,
}

impl GPUMemoriesAccess for GPU {
    fn read_oam(&mut self, addr: u16) -> u8 {
        self.oam[addr as usize]
    }
    fn write_oam(&mut self, addr: u16, byte: u8) {
        self.oam[addr as usize] = byte;
    }
    fn gpu_mode(&self) -> u8 {
        self.mode
    }
    fn get_line(&self) -> u8 {
        self.line
    }
    fn oam_scan_row(&self) -> u8 {
        self.accessed_oam_row
    }
    fn is_lcd_enabled(&self) -> bool {
        self.lcd_enabled
    }
    fn cgb_mode(&self) -> bool {
        self.cgb_mode
    }
    fn take_hblank(&mut self) -> bool {
        let v = self.hblank_flag;
        self.hblank_flag = false;
        v
    }
    fn oam_accessible(&self) -> bool {
        !self.lcd_enabled || (self.mode != 2 && self.mode != 3)
    }
    fn apply_oam_corruption(&mut self, r: u8) {
        if !self.lcd_enabled {
            return;
        }
        let r = r as usize;
        if !(8..=152).contains(&r) {
            return;
        }
        let a = self.oam[r] as u16 | ((self.oam[r + 1] as u16) << 8);
        let b = self.oam[r - 8] as u16 | ((self.oam[r - 7] as u16) << 8);
        let c = self.oam[r - 4] as u16 | ((self.oam[r - 3] as u16) << 8);
        let result = ((a ^ c) & (b ^ c)) ^ c;
        self.oam[r] = result as u8;
        self.oam[r + 1] = (result >> 8) as u8;
        for i in 2..8usize {
            self.oam[r + i] = self.oam[r - 8 + i];
        }
    }
    // POP read corruption (GB_trigger_oam_bug_read in SameBoy).
    // Each formula variant modifies the scan row (r-8), then scan row is always
    // copied to the corrupted row (r) — matches the unconditional copy in SameBoy.
    fn apply_oam_read_corruption(&mut self, r: u8) {
        if !self.lcd_enabled {
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
                    let a = self.oam[r + i];
                    let b = self.oam[r - 8 + i];
                    let c = self.oam[r - 4 + i];
                    self.oam[r - 8 + i] = b | (a & c);
                }
            }
            // Secondary: formula on scan row bytes 0-1; copy scan row → prev2
            0x10 => {
                for i in 0..2usize {
                    let a = self.oam[r - 16 + i];
                    let b = self.oam[r - 8 + i];
                    let c = self.oam[r + i];
                    let d = self.oam[r - 4 + i];
                    self.oam[r - 8 + i] = (b & (a | c | d)) | (a & c & d);
                }
                for i in 0..8usize {
                    self.oam[r - 16 + i] = self.oam[r - 8 + i];
                }
            }
            // Tertiary/quaternary: formula on scan row bytes 0-1; copy scan row → prev2 and prev4
            0x00 => {
                for i in 0..2usize {
                    let a = self.oam[r + i];
                    let b = self.oam[r - 4 + i];
                    let c = self.oam[r - 8 + i];
                    let d = self.oam[r - 16 + i];
                    let e = self.oam[r - 32 + i];
                    self.oam[r - 8 + i] = match r {
                        0x20 => (c & (a | b | d | e)) | (a & b & d & e), // tertiary_2
                        0x40 => {
                            // quaternary_dmg
                            // SameBoy: (e & (h|g|(~d&f)|c|b)) | (c&g&h)
                            // where b=oam[r+i], c=oam[r-4+i], d=oam[r-6+i], e=oam[r-8+i],
                            //       f=oam[r-14+i], g=oam[r-16+i], h=oam[r-32+i]
                            let sb_d = self.oam[r - 6 + i];
                            let sb_f = self.oam[r - 14 + i];
                            // my c=sb_e, my b=sb_c, my a=sb_b, my d=sb_g, my e=sb_h
                            (c & (e | d | ((!sb_d) & sb_f) | b | a)) | (b & d & e)
                        }
                        0x60 => (c & (a | b | d | e)) | (b & d & e), // tertiary_3
                        _ => c | (a & b & d & e),                    // tertiary_1 (r==0x80)
                    };
                }
                for i in 0..8usize {
                    self.oam[r - 16 + i] = self.oam[r - 8 + i];
                    self.oam[r - 32 + i] = self.oam[r - 8 + i];
                }
            }
            _ => return,
        }
        // Always: copy (possibly modified) scan row → corrupted row
        for i in 0..8usize {
            self.oam[r + i] = self.oam[r - 8 + i];
        }
        if r == 0x80 {
            for i in 0..8usize {
                self.oam[i] = self.oam[0x80 + i];
            }
        }
    }
    fn read_vram(&mut self, addr: u16) -> u8 {
        let offset = self.vram_bank as usize * 8192;
        self.vram[offset + addr as usize]
    }
    fn write_vram(&mut self, addr: u16, byte: u8) {
        let offset = self.vram_bank as usize * 8192;
        self.vram[offset + addr as usize] = byte;
    }
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF40 => {
                (if if self.cgb_mode {
                    self.bg_master_priority
                } else {
                    self.bg_enabled
                } {
                    0x01
                } else {
                    0
                }) | (if self.obj_enabled { 0x02 } else { 0 })
                    | (if self.obj_size { 0x04 } else { 0 })
                    | (if self.bg_map { 0x08 } else { 0 })
                    | (if self.bg_tile { 0x10 } else { 0 })
                    | (if self.window_enabled { 0x20 } else { 0 })
                    | (if self.window_map { 0x40 } else { 0 })
                    | (if self.lcd_enabled { 0x80 } else { 0 })
            }
            0xFF41 => {
                let mode_bits = if self.lcd_startup_ticks > 0 {
                    0
                } else {
                    self.mode & 0x03
                };
                0x80 | mode_bits
                    | (if self.compare_enabled { 0x40 } else { 0 })
                    | (if self.mode2_int_enabled { 0x20 } else { 0 })
                    | (if self.mode1_int_enabled { 0x10 } else { 0 })
                    | (if self.mode0_int_enabled { 0x08 } else { 0 })
                    | (if self.lyc_flag { 0x04 } else { 0 })
            }
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.line,
            0xFF45 => self.compare_line,
            0xFF47 => self.bg_palette.byte,
            0xFF48 => self.obj_palette_0.byte,
            0xFF49 => self.obj_palette_1.byte,
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            0xFF4F if self.cgb_mode => self.vram_bank | 0xFE,
            0xFF68 if self.cgb_mode => self.bcps,
            0xFF69 if self.cgb_mode => self.bg_palette_data[(self.bcps & 0x3F) as usize],
            0xFF6A if self.cgb_mode => self.ocps,
            0xFF6B if self.cgb_mode => self.obj_palette_data[(self.ocps & 0x3F) as usize],
            _ => 0xFF,
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr {
            0xFF40 => {
                // LCD Control
                // bit 0: DMG = bg_enable; CGB = BG/window master priority (BG always drawn)
                if self.cgb_mode {
                    self.bg_master_priority = (byte & 0x01) != 0;
                } else {
                    self.bg_enabled = (byte & 0x01) != 0;
                }
                self.obj_enabled = (byte & 0x02) != 0;
                self.obj_size = (byte & 0x04) != 0;
                self.bg_map = (byte & 0x08) != 0;
                self.bg_tile = (byte & 0x10) != 0;
                self.window_enabled = (byte & 0x20) != 0;
                self.window_map = (byte & 0x40) != 0;
                let was_enabled = self.lcd_enabled;
                self.lcd_enabled = (byte & 0x80) != 0;
                if !was_enabled && self.lcd_enabled {
                    self.mode = 2;
                    self.line = 0;
                    self.modeclock = 4; // hardware starts mode 2 ~1 M-cycle in, not at T=0
                    self.lyc_flag = self.line == self.compare_line;
                    // 16T: compensates for the write firing "early" (at T1 of the write M-cycle)
                    // rather than at T4, leaving the mode-0 window visible for the next STAT read.
                    self.lcd_startup_ticks = 16;
                    let new_stat = self.compute_stat_line();
                    if !self.stat_line && new_stat {
                        self.pending_stat = true;
                    }
                    self.stat_line = new_stat;
                } else if was_enabled && !self.lcd_enabled {
                    self.mode = 0;
                    self.line = 0;
                    self.modeclock = 0;
                    self.lcd_startup_ticks = 0;
                    // lyc_flag intentionally NOT updated here — frozen at last value
                    // stat_line = lyc_source only (mode sources gated by lcd_enabled)
                    self.stat_line = self.compute_stat_line();
                }
            }
            0xFF41 => {
                let old_stat = self.stat_line;
                self.compare_enabled = (byte & 0x40) != 0;
                self.mode2_int_enabled = (byte & 0x20) != 0;
                self.mode1_int_enabled = (byte & 0x10) != 0;
                self.mode0_int_enabled = (byte & 0x08) != 0;
                self.stat_line = self.compute_stat_line();
                if !old_stat && self.stat_line {
                    self.pending_stat = true;
                }
            }
            0xFF42 => {
                self.scroll_y = byte;
            }
            0xFF43 => {
                self.scroll_x = byte;
            }
            0xFF44 => {
                self.line = 0;
            }
            0xFF45 => {
                let old_stat = self.stat_line;
                self.compare_line = byte;
                if self.lcd_enabled {
                    self.lyc_flag = self.line == self.compare_line;
                }
                self.stat_line = self.compute_stat_line();
                if !old_stat && self.stat_line {
                    self.pending_stat = true;
                }
            }
            0xFF46 => {
                // DMA transfer, handled from outside
            }
            0xFF47 => {
                self.bg_palette.update(byte);
                if !self.cgb_mode {
                    self.bg_colors[0] =
                        Self::dmg_colors_from_palette(&self.bg_palette, &self.dmg_palette);
                }
            }
            0xFF48 => {
                self.obj_palette_0.update(byte);
                if !self.cgb_mode {
                    self.obj_colors[0] =
                        Self::dmg_colors_from_palette(&self.obj_palette_0, &self.dmg_palette);
                }
            }
            0xFF49 => {
                self.obj_palette_1.update(byte);
                if !self.cgb_mode {
                    self.obj_colors[1] =
                        Self::dmg_colors_from_palette(&self.obj_palette_1, &self.dmg_palette);
                }
            }
            0xFF4A => {
                self.window_y = byte;
            }
            0xFF4B => {
                self.window_x = byte;
            }
            0xFF4F if self.cgb_mode => {
                self.vram_bank = byte & 0x01;
            }
            0xFF68 => {
                self.bcps = byte;
            }
            0xFF69 => {
                let index = self.bcps & 0x3F;
                self.bg_palette_data[index as usize] = byte;
                Self::recompute_cgb_color(&self.bg_palette_data, &mut self.bg_colors, index);
                if self.bcps & 0x80 != 0 {
                    self.bcps = (self.bcps & 0x80) | ((index + 1) & 0x3F);
                }
            }
            0xFF6A => {
                self.ocps = byte;
            }
            0xFF6B => {
                let index = self.ocps & 0x3F;
                self.obj_palette_data[index as usize] = byte;
                Self::recompute_cgb_color(&self.obj_palette_data, &mut self.obj_colors, index);
                if self.ocps & 0x80 != 0 {
                    self.ocps = (self.ocps & 0x80) | ((index + 1) & 0x3F);
                }
            }
            _ => {}
        }
    }
    fn step(&mut self, t: u8) -> (bool, bool) {
        GPU::step(self, t)
    }
    fn post_boot_init(&mut self) {
        self.write_byte(0xFF40, 0x91);
        self.write_byte(0xFF47, 0xFC);
        self.mode = 1;
        self.line = 153;
        self.modeclock = 396;
        self.lyc_flag = self.line == self.compare_line;
        self.lcd_startup_ticks = 0;
    }
}

impl GPU {
    pub fn new(cgb_mode: bool) -> Self {
        let dmg_palette = [0x00C4F0C2u32, 0x005AB9A8, 0x001E606E, 0x002D1B00];
        let bg_pal = Palette::new();
        let obj_pal_0 = Palette::new();
        let obj_pal_1 = Palette::new();
        let mut bg_colors = [[0u32; 4]; 8];
        let mut obj_colors = [[0u32; 4]; 8];
        bg_colors[0] = Self::dmg_colors_from_palette(&bg_pal, &dmg_palette);
        obj_colors[0] = Self::dmg_colors_from_palette(&obj_pal_0, &dmg_palette);
        obj_colors[1] = Self::dmg_colors_from_palette(&obj_pal_1, &dmg_palette);
        GPU {
            vram: [0; 16384],
            oam: [0; 160],
            buffer: [0; 160 * 144],
            cgb_mode,
            vram_bank: 0,
            bg_palette_data: [0; 64],
            obj_palette_data: [0; 64],
            bcps: 0,
            ocps: 0,
            bg_colors,
            obj_colors,
            dmg_palette,
            modeclock: 0,
            mode: 2,
            line: 0,
            bg_enabled: false,
            bg_master_priority: true,
            obj_enabled: false,
            obj_size: false,
            bg_map: false,
            bg_tile: false,
            window_enabled: false,
            window_map: false,
            lcd_enabled: false,
            compare_enabled: false,
            compare_line: 0,
            hblank_flag: false,
            mode0_int_enabled: false,
            mode1_int_enabled: false,
            mode2_int_enabled: false,
            accessed_oam_row: 0xFF,
            mode3_extra: 0,
            stat_line: false,
            pending_stat: false,
            lyc_flag: false,
            lcd_startup_ticks: 0,
            scroll_x: 0,
            scroll_y: 0,
            bg_palette: bg_pal,
            obj_palette_0: obj_pal_0,
            obj_palette_1: obj_pal_1,
            window_x: 0,
            window_y: 0,
        }
    }

    fn compare(&self) -> bool {
        self.line == self.compare_line
    }

    pub fn get_buffer(&self) -> &[u32; 160 * 144] {
        &self.buffer
    }

    pub fn set_dmg_palette(&mut self, palette: [u32; 4]) {
        self.dmg_palette = palette;
        if !self.cgb_mode {
            self.bg_colors[0] = Self::dmg_colors_from_palette(&self.bg_palette, &self.dmg_palette);
            self.obj_colors[0] =
                Self::dmg_colors_from_palette(&self.obj_palette_0, &self.dmg_palette);
            self.obj_colors[1] =
                Self::dmg_colors_from_palette(&self.obj_palette_1, &self.dmg_palette);
        }
    }

    fn dmg_colors_from_palette(palette: &Palette, dmg_palette: &[u32; 4]) -> [u32; 4] {
        let mut colors = [0u32; 4];
        for cn in 0u8..4 {
            colors[cn as usize] = dmg_palette[palette.get(cn) as usize];
        }
        colors
    }

    fn color15_to_xrgb(c: u16) -> u32 {
        let expand = |v: u8| (v << 3) | (v >> 2);
        let r = expand((c & 0x1F) as u8);
        let g = expand(((c >> 5) & 0x1F) as u8);
        let b = expand(((c >> 10) & 0x1F) as u8);
        ((r as u32) << 16) | ((g as u32) << 8) | b as u32
    }

    fn recompute_cgb_color(palette_data: &[u8; 64], colors: &mut [[u32; 4]; 8], index: u8) {
        let raw_index = (index & !1) as usize;
        let c = u16::from_le_bytes([palette_data[raw_index], palette_data[raw_index + 1]]);
        let palette_idx = (index / 8) as usize;
        let colour_num = ((index % 8) / 2) as usize;
        colors[palette_idx][colour_num] = Self::color15_to_xrgb(c);
    }

    fn get_tileset_index(&self, mut index: u8) -> usize {
        let mut offset: usize = if self.bg_tile {
            TILEDATA1_OFFSET
        } else {
            TILEDATA0_OFFSET
        };

        if index >= 128 {
            offset = TILEDATA_SHARED;
            index -= 128;
        }

        offset + 2 * TILE_SIZE * (index as usize)
    }

    // Decode one BG/window tile pixel. Returns (colour_number, XRGB8888 color, bg_priority).
    fn resolve_tile_pixel(
        &self,
        tilemap_index: usize,
        cell_x: usize,
        cell_y: usize,
    ) -> (u8, u32, bool) {
        let tile_id = self.vram[tilemap_index]; // bank 0
        let attr = self.vram[8192 + tilemap_index]; // bank 1 (always 0 in DMG)
        let palette_idx = (attr & 0x07) as usize;
        let tile_bank = ((attr >> 3) & 0x01) as usize;
        let flip_h = (attr >> 5) & 0x01 != 0;
        let flip_v = (attr >> 6) & 0x01 != 0;
        let bg_priority = (attr >> 7) & 0x01 != 0;

        let effective_cell_y = if flip_v { 7 - cell_y } else { cell_y };
        let bit_pos = if flip_h {
            cell_x as u8
        } else {
            7 - cell_x as u8
        };
        let bank_offset = tile_bank * 8192;

        let tileset_index = self.get_tileset_index(tile_id) + 2 * effective_cell_y;
        let byte_1 = self.vram[bank_offset + tileset_index];
        let byte_2 = self.vram[bank_offset + tileset_index + 1];

        let high_bit = is_bit_set(bit_pos, byte_2 as u16) as u8;
        let low_bit = is_bit_set(bit_pos, byte_1 as u16) as u8;
        let colour_number = (high_bit << 1) | low_bit;

        (
            colour_number,
            self.bg_colors[palette_idx][colour_number as usize],
            bg_priority,
        )
    }

    // draws a line on the buffer
    pub fn render_scan_to_buffer(&mut self) {
        let line_to_draw: usize = self.line.wrapping_add(self.scroll_y) as usize;

        // save colour numbers being rendered before palette application (bits 0-1),
        // plus bg-priority flag from CGB tile attr in bit 2. 0 colour number = transparent.
        let mut rendering_row = [0u8; 160];

        // background (always drawn in CGB mode regardless of LCDC bit 0)
        if self.cgb_mode || self.bg_enabled {
            let tilemap_offset = if self.bg_map {
                TILEMAP1_OFFSET
            } else {
                TILEMAP0_OFFSET
            };

            // the row of the cell in the tilemap
            let tilemap_y: usize = (line_to_draw / TILE_SIZE) % TILES_IN_A_TILEMAP_COL;

            // the row of the pixel in the cell
            let cell_y: usize = line_to_draw % TILE_SIZE;

            // for each pixel in the line (which is long 160 pixel)
            #[allow(clippy::needless_range_loop)]
            for row_pixel in 0..TILES_IN_A_SCREEN_ROW * TILE_SIZE {
                let curr_pixel_x = self.scroll_x as usize + row_pixel;

                // the col of the cell in the tilemap
                let tilemap_x: usize = (curr_pixel_x / TILE_SIZE) % TILES_IN_A_TILEMAP_ROW;

                // the col of the pixel in the cell
                let cell_x: usize = curr_pixel_x % TILE_SIZE;

                // find the tile in the vram
                let tilemap_index =
                    tilemap_offset + (tilemap_y * TILES_IN_A_TILEMAP_ROW + tilemap_x);

                let (colour_number, color, bg_priority) =
                    self.resolve_tile_pixel(tilemap_index, cell_x, cell_y);

                rendering_row[row_pixel] = colour_number | ((bg_priority as u8) << 2);
                let index = (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + row_pixel;
                self.buffer[index] = color;
            }
        }

        // window
        if self.window_enabled && self.window_y <= self.line {
            // window_x is treated as 7 if it's anywhere from 0-6
            let window_x = (if self.window_x < 7 { 7 } else { self.window_x }).wrapping_sub(7);
            let tilemap_offset = if self.window_map {
                TILEMAP1_OFFSET
            } else {
                TILEMAP0_OFFSET
            };

            let window_line: usize = self.line.wrapping_sub(self.window_y) as usize;
            let tilemap_y: usize = (window_line / TILE_SIZE) % TILES_IN_A_TILEMAP_COL;
            let cell_y: usize = window_line % TILE_SIZE;

            #[allow(clippy::needless_range_loop)]
            for pixel in (window_x as usize)..TILES_IN_A_SCREEN_ROW * TILE_SIZE {
                let mut curr_pixel_x = (pixel as u8).wrapping_add(self.scroll_x);
                if curr_pixel_x >= window_x {
                    curr_pixel_x = pixel as u8 - window_x;
                }

                let tilemap_x: usize = (curr_pixel_x as usize / TILE_SIZE) % TILES_IN_A_TILEMAP_ROW;
                let cell_x: usize = curr_pixel_x as usize % TILE_SIZE;
                let tilemap_index =
                    tilemap_offset + (tilemap_y * TILES_IN_A_TILEMAP_ROW + tilemap_x);

                let (colour_number, color, bg_priority) =
                    self.resolve_tile_pixel(tilemap_index, cell_x, cell_y);

                rendering_row[pixel] = colour_number | ((bg_priority as u8) << 2);
                let index = (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + pixel;
                self.buffer[index] = color;
            }
        }

        // sprites
        if self.obj_enabled {
            let sprite_height: u8 = if self.obj_size { 16 } else { 8 };

            let mut sprite_occupied = [false; 160usize];

            for sprite_num in 0..40usize {
                let base = sprite_num * 4;
                let y = self.oam[base].wrapping_sub(16);
                let x = self.oam[base + 1].wrapping_sub(8);
                let pos_base = self.oam[base + 2];
                let opt = self.oam[base + 3];

                let flip_y = (opt & 0x40) != 0;
                let flip_x = (opt & 0x20) != 0;
                let z = (opt & 0x80) != 0;
                // CGB mode: bits 0-2 = palette index (0-7); DMG mode: bit 4 = OBP0/OBP1
                let cgb_palette = if self.cgb_mode {
                    (opt & 0x07) as usize
                } else {
                    ((opt >> 4) & 0x01) as usize
                };
                // CGB mode: bit 3 = tile VRAM bank; DMG: always bank 0
                let tile_vbank = if self.cgb_mode {
                    ((opt >> 3) & 0x01) as usize
                } else {
                    0
                };

                // not intersecting with scanline, don't draw
                if self.line.wrapping_sub(y) >= sprite_height {
                    continue;
                }

                let mut pos = pos_base;

                // handle upside down
                let mut sprite_pixel_row = if flip_y {
                    sprite_height - self.line.wrapping_sub(y) - 1
                } else {
                    self.line.wrapping_sub(y)
                };

                // go to next tile if we have to render 2nd part of the 16pixel sprite
                if sprite_pixel_row >= 8 {
                    pos = pos.wrapping_add(1);
                    sprite_pixel_row -= 8;
                }

                // sprites always use tiledata1; tile_vbank selects VRAM bank
                let bank_offset = tile_vbank * 8192;
                let tile_in_tileset: usize =
                    TILEDATA1_OFFSET + (2 * 8 * pos as usize + sprite_pixel_row as usize * 2);

                // a tile pixel line is encoded in two consecutive bytes
                let byte_1 = self.vram[bank_offset + tile_in_tileset];
                let byte_2 = self.vram[bank_offset + tile_in_tileset + 1];

                for pixel in 0..8u8 {
                    let ix = if flip_x { pixel } else { 7 - pixel };

                    let curr_x = x.wrapping_add(7 - pixel);

                    // out of the line, don't draw
                    if curr_x >= 160 {
                        continue;
                    }

                    let high_bit: u8 = is_bit_set(7 - ix, byte_2 as u16) as u8;
                    let low_bit: u8 = is_bit_set(7 - ix, byte_1 as u16) as u8;

                    let colour_number = (high_bit << 1) | low_bit;

                    // transparent, don't draw
                    if colour_number == 0 {
                        continue;
                    }

                    // bg-priority tile attr (CGB) or z-order with non-transparent bg pixel
                    // In CGB mode with master priority off, sprites always win
                    let bg_wins = if self.cgb_mode && !self.bg_master_priority {
                        false
                    } else {
                        (rendering_row[curr_x as usize] & 0x04 != 0)
                            || (z && rendering_row[curr_x as usize] & 0x03 != 0)
                    };
                    if bg_wins {
                        continue;
                    }

                    // lower OAM index wins over higher index
                    if sprite_occupied[curr_x as usize] {
                        continue;
                    }
                    sprite_occupied[curr_x as usize] = true;

                    let colour = self.obj_colors[cgb_palette][colour_number as usize];
                    let index: usize =
                        (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + curr_x as usize;
                    self.buffer[index] = colour;
                }
            }
        }
    }

    fn compute_stat_line(&self) -> bool {
        // LYC source is NOT gated by lcd_enabled: it stays active even when LCD is off,
        // so disabling/re-enabling LCD with the same lyc_flag doesn't cause a spurious
        // 0→1 transition (and therefore no spurious STAT interrupt).
        let lyc_source = self.compare_enabled && self.lyc_flag;
        let mode_sources = self.lcd_enabled
            && ((self.mode0_int_enabled && self.mode == 0)
                || (self.mode1_int_enabled && self.mode == 1)
                || (self.mode2_int_enabled && self.mode == 2));
        lyc_source || mode_sources
    }

    fn collect_scanline_sprite_xs(&self) -> ([u8; 10], usize) {
        let mut xs = [0u8; 10];
        let mut count = 0usize;
        if !self.obj_enabled {
            return (xs, 0);
        }
        let sprite_height: u8 = if self.obj_size { 16 } else { 8 };
        for i in 0..40usize {
            let y = self.oam[i * 4].wrapping_sub(16);
            if self.line.wrapping_sub(y) < sprite_height {
                let x = self.oam[i * 4 + 1];
                if x < 168 {
                    xs[count] = x;
                    count += 1;
                    if count >= 10 {
                        break;
                    }
                }
            }
        }
        (xs, count)
    }

    fn compute_mode3_sprite_penalty(xs: &[u8]) -> u16 {
        let mut total: u16 = 0;
        let mut buckets = [0i16; 22]; // covers x 0..=167 → bucket 0..=20

        for &x in xs {
            if x >= 168 {
                continue;
            }
            let bucket = (x >> 3) as usize;
            let alignment_penalty = 5i16 - (x & 7) as i16;
            if alignment_penalty > buckets[bucket] {
                buckets[bucket] = alignment_penalty;
            }
            total += 6;
        }

        for &b in &buckets {
            if b > 0 {
                total += b as u16;
            }
        }

        total & !3 // round down to nearest multiple of 4 → T-cycles
    }

    // go forward based on the cpu's last operation clocks
    pub fn step(&mut self, t: u8) -> (bool, bool) {
        if !self.lcd_enabled {
            return (false, false);
        }
        self.lcd_startup_ticks = self.lcd_startup_ticks.saturating_sub(t);
        self.modeclock += t as u16;

        let mut vblank_interrupt = false;
        let mut stat_interrupt = false;

        if self.pending_stat {
            self.pending_stat = false;
            stat_interrupt = true;
        }

        let old_stat = self.stat_line;

        match self.mode {
            // scanline, oam read mode
            2 => {
                self.accessed_oam_row = (self.modeclock / 4 * 8) as u8;
                if self.modeclock >= 80 {
                    self.modeclock = 0;
                    self.mode = 3;
                    self.accessed_oam_row = 0xFF;
                    let (xs, count) = self.collect_scanline_sprite_xs();
                    self.mode3_extra = Self::compute_mode3_sprite_penalty(&xs[..count]);
                }
            }
            // scanline, vram read mode
            3 => {
                self.accessed_oam_row = 0xFF;
                if self.modeclock >= 172 + self.mode3_extra {
                    self.modeclock = 0;
                    self.mode = 0;
                    self.hblank_flag = true;
                    self.render_scan_to_buffer();
                }
            }
            // hblank
            0 => {
                self.accessed_oam_row = 0xFF;
                if self.modeclock >= 204 - self.mode3_extra {
                    self.modeclock = 0;
                    self.line += 1;
                    self.lyc_flag = self.line == self.compare_line;

                    if self.line == 144 {
                        self.mode = 1;
                        vblank_interrupt = true;
                    } else {
                        self.mode = 2;
                        self.accessed_oam_row = 0;
                    }
                }
            }
            // vblank (10 lines)
            1 => {
                self.accessed_oam_row = 0xFF;
                if self.modeclock >= 456 {
                    self.modeclock = 0;
                    self.line += 1;

                    if self.line > 153 {
                        self.mode = 2;
                        self.accessed_oam_row = 0;
                        self.line = 0;
                    }
                    self.lyc_flag = self.line == self.compare_line;
                }
            }
            _ => panic!("Sorry what?"),
        }

        self.stat_line = self.compute_stat_line();
        if !old_stat && self.stat_line {
            stat_interrupt = true;
        }

        (vblank_interrupt, stat_interrupt)
    }
}

impl Default for GPU {
    fn default() -> Self {
        GPU::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // test scroll_y write and read access, as well as the default value
    #[test]
    fn test_scroll_y() {
        let mut gpu = GPU::new(false);

        assert_eq!(gpu.scroll_y, 0);

        gpu.write_byte(0xFF42, 1);

        assert_eq!(gpu.scroll_y, 1);
        assert_eq!(gpu.read_byte(0xFF42), 1);
    }

    // test scroll_x write and read access, as well as the default value
    #[test]
    fn test_scroll_x() {
        let mut gpu = GPU::new(false);

        assert_eq!(gpu.scroll_x, 0);

        gpu.write_byte(0xFF43, 1);

        assert_eq!(gpu.scroll_x, 1);
        assert_eq!(gpu.read_byte(0xFF43), 1);
    }

    // test palette write and read access, as well as the default value
    #[test]
    fn test_bg_palette() {
        let mut gpu = GPU::new(false);

        // default value
        assert_eq!(gpu.bg_palette.byte, 0xFF);

        gpu.write_byte(0xFF47, 1);

        assert_eq!(gpu.bg_palette.byte, 1);
    }

    // test obj palette 0 write and read access, as well as the default value
    #[test]
    fn test_obj_palette_0() {
        let mut gpu = GPU::new(false);

        // default value
        assert_eq!(gpu.obj_palette_0.byte, 0xFF);

        gpu.write_byte(0xFF48, 1);

        assert_eq!(gpu.obj_palette_0.byte, 1);
    }

    // test palette write and read access, as well as the default value
    #[test]
    fn test_obj_palette_1() {
        let mut gpu = GPU::new(false);

        // default value
        assert_eq!(gpu.obj_palette_1.byte, 0xFF);

        gpu.write_byte(0xFF49, 1);

        assert_eq!(gpu.obj_palette_1.byte, 1);
    }

    #[test]
    fn test_window_x_y() {
        let mut gpu = GPU::new(false);

        // default value
        assert_eq!(gpu.window_y, 0);
        assert_eq!(gpu.window_x, 0);

        gpu.write_byte(0xFF4A, 1);
        gpu.write_byte(0xFF4B, 2);

        assert_eq!(gpu.window_y, 1);
        assert_eq!(gpu.window_x, 2);
    }

    // test control write and read access, as well as the default value
    #[test]
    fn test_control() {
        let mut gpu = GPU::new(false);

        assert!(!gpu.bg_enabled);
        assert!(!gpu.obj_enabled);
        assert!(!gpu.obj_size);
        assert!(!gpu.bg_map);
        assert!(!gpu.bg_tile);
        assert!(!gpu.window_enabled);
        assert!(!gpu.window_map);
        assert!(!gpu.lcd_enabled);

        gpu.write_byte(0xFF40, 1);
        assert!(gpu.bg_enabled);
        assert_eq!(gpu.read_byte(0xFF40), 1);

        gpu.write_byte(0xFF40, 0x02);
        assert!(gpu.obj_enabled);
        assert_eq!(gpu.read_byte(0xFF40), 0x02);

        gpu.write_byte(0xFF40, 0x04);
        assert!(gpu.obj_size);
        assert_eq!(gpu.read_byte(0xFF40), 0x04);

        gpu.write_byte(0xFF40, 0x08);
        assert!(gpu.bg_map);
        assert_eq!(gpu.read_byte(0xFF40), 0x08);

        gpu.write_byte(0xFF40, 0x10);
        assert!(gpu.bg_tile);
        assert_eq!(gpu.read_byte(0xFF40), 0x10);

        gpu.write_byte(0xFF40, 0x20);
        assert!(gpu.window_enabled);
        assert_eq!(gpu.read_byte(0xFF40), 0x20);

        gpu.write_byte(0xFF40, 0x40);
        assert!(gpu.window_map);
        assert_eq!(gpu.read_byte(0xFF40), 0x40);

        gpu.write_byte(0xFF40, 0x80);
        assert!(gpu.lcd_enabled);
        assert_eq!(gpu.read_byte(0xFF40), 0x80);
    }

    // test line read and write access
    #[test]
    fn test_line() {
        let mut gpu = GPU::new(false);

        assert_eq!(gpu.line, 0);
        gpu.write_byte(0xFF44, 1);
        // no write access
        assert_eq!(gpu.line, 0);

        gpu.line = 15;
        assert_eq!(gpu.read_byte(0xFF44), 15);
    }

    // test OAM write and read
    #[test]
    fn test_sprite() {
        let mut gpu = GPU::new(false);

        gpu.write_oam(0, 18);
        assert_eq!(gpu.read_oam(0), 18);

        gpu.write_oam(1, 14);
        assert_eq!(gpu.read_oam(1), 14);

        gpu.write_oam(2, 4);
        assert_eq!(gpu.read_oam(2), 4);

        gpu.write_oam(3, 0b10000000);
        assert_eq!(gpu.read_oam(3), 0b10000000);

        gpu.write_oam(159, 0b00010000);
        assert_eq!(gpu.read_oam(159), 0b00010000);
    }
}
