use crate::cpu::is_bit_set;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

// --- Screen / framebuffer geometry ---
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;
const FRAMEBUFFER_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

// --- Tile / tilemap geometry ---
const TILE_SIZE: usize = 8;
const BYTES_PER_TILE_ROW: usize = 2;
const TILES_IN_A_TILEMAP_ROW: usize = 32;
const TILES_IN_A_TILEMAP_COL: usize = 32;

// --- VRAM layout (offsets within an 8 KiB bank) ---
const VRAM_BANK_SIZE: usize = 8192;
const TILEMAP0_OFFSET: usize = 0x9800 - 0x8000;
const TILEMAP1_OFFSET: usize = 0x9C00 - 0x8000;
const TILEDATA1_OFFSET: usize = 0;
const TILEDATA0_OFFSET: usize = 0x9000 - 0x8000;
const TILEDATA_SHARED: usize = 0x8800 - 0x8000; // when tile index >= 128

// --- PPU timing (T-cycles / dots) ---
const MODE2_DOTS: u16 = 80;
const MODE3_BASE_DOTS: u16 = 172;
const MODE0_BASE_DOTS: u16 = 204;
const SCANLINE_DOTS: u16 = 456;
const VBLANK_START_LINE: u8 = 144;
const LAST_LINE: u8 = 153;

const COLOR_CORRECTION: bool = cfg!(feature = "color-correction");

// serde default helpers for per-scanline scratch arrays
fn arr_u8_160() -> [u8; SCREEN_WIDTH] {
    [0u8; SCREEN_WIDTH]
}
fn arr_bool_160() -> [bool; SCREEN_WIDTH] {
    [false; SCREEN_WIDTH]
}
fn arr_sprite_10() -> [SpriteData; 10] {
    [SpriteData::default(); 10]
}

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

/// One OAM sprite entry cached for per-dot rendering.
#[derive(Copy, Clone, Default, Serialize, Deserialize)]
struct SpriteData {
    y: u8,    // raw OAM byte 0 (screen_y = y - 16)
    x: u8,    // raw OAM byte 1 (screen_x = x - 8)
    tile: u8, // raw OAM byte 2
    attr: u8, // raw OAM byte 3
}

#[derive(Serialize, Deserialize)]
pub struct GPU {
    #[serde(with = "BigArray")]
    pub vram: [u8; 16384],
    #[serde(with = "BigArray")]
    pub oam: [u8; 160],
    #[serde(with = "BigArray")]
    buffer: [u32; FRAMEBUFFER_LEN],

    // CGB support
    pub cgb_mode: bool,
    #[serde(default)]
    pub dmg_compat: bool, // DMG game running on GBC hardware (boot ROM colorization)
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
    #[serde(default)]
    compat_bg_ref: [u32; 4], // CGB BG palette 0 colors set by boot ROM
    #[serde(default)]
    compat_obj_ref: [[u32; 4]; 2], // CGB OBJ palettes 0+1 set by boot ROM

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
    window_line: u8,          // internal counter: increments per rendered window scanline
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

    // Per-dot rendering state (rebuilt every scanline at Mode 2→3 transition)
    #[serde(default)]
    dot_x: u8, // next pixel to output in Mode 3 (0..=159)
    #[serde(default)]
    scan_sprite_count: u8,
    #[serde(default = "arr_sprite_10")]
    scan_sprites: [SpriteData; 10],
    #[serde(default = "arr_u8_160", with = "BigArray")]
    bg_row: [u8; SCREEN_WIDTH], // colour_number | (bg_priority << 2) for each pixel this line
    #[serde(default = "arr_bool_160", with = "BigArray")]
    sprite_occupied: [bool; SCREEN_WIDTH], // first sprite wins per pixel position
    #[serde(default)]
    window_triggered: bool, // true if any window pixel was rendered this scanline
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
        crate::oam_dma::apply_oam_corruption(&mut self.oam, self.lcd_enabled, r);
    }
    fn apply_oam_read_corruption(&mut self, r: u8) {
        crate::oam_dma::apply_oam_read_corruption(&mut self.oam, self.lcd_enabled, r);
    }
    fn read_vram(&mut self, addr: u16) -> u8 {
        let offset = self.vram_bank as usize * VRAM_BANK_SIZE;
        self.vram[offset + addr as usize]
    }
    fn write_vram(&mut self, addr: u16, byte: u8) {
        let offset = self.vram_bank as usize * VRAM_BANK_SIZE;
        self.vram[offset + addr as usize] = byte;
    }
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.read_lcdc(),
            0xFF41 => self.read_stat(),
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
            0xFF40 => self.write_lcdc(byte),
            0xFF41 => self.write_stat(byte),
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
                if let Some(c) = self.dmg_palette_colors(&self.bg_palette, &self.compat_bg_ref) {
                    self.bg_colors[0] = c;
                }
            }
            0xFF48 => {
                self.obj_palette_0.update(byte);
                if let Some(c) =
                    self.dmg_palette_colors(&self.obj_palette_0, &self.compat_obj_ref[0])
                {
                    self.obj_colors[0] = c;
                }
            }
            0xFF49 => {
                self.obj_palette_1.update(byte);
                if let Some(c) =
                    self.dmg_palette_colors(&self.obj_palette_1, &self.compat_obj_ref[1])
                {
                    self.obj_colors[1] = c;
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
                let index = Self::write_cgb_palette(
                    &mut self.bg_palette_data,
                    &mut self.bg_colors,
                    &mut self.bcps,
                    byte,
                );
                // DMG-compat: snapshot boot-ROM BG palette 0 as the reference for FF47.
                if self.dmg_compat && (index / 8) == 0 {
                    let colour_num = ((index % 8) / 2) as usize;
                    self.compat_bg_ref[colour_num] = self.bg_colors[0][colour_num];
                }
            }
            0xFF6A => {
                self.ocps = byte;
            }
            0xFF6B => {
                let index = Self::write_cgb_palette(
                    &mut self.obj_palette_data,
                    &mut self.obj_colors,
                    &mut self.ocps,
                    byte,
                );
                // DMG-compat: snapshot boot-ROM OBJ palettes 0+1 as references for FF48/FF49.
                if self.dmg_compat {
                    let palette_idx = (index / 8) as usize;
                    let colour_num = ((index % 8) / 2) as usize;
                    if palette_idx < 2 {
                        self.compat_obj_ref[palette_idx][colour_num] =
                            self.obj_colors[palette_idx][colour_num];
                    }
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
            buffer: [0; FRAMEBUFFER_LEN],
            cgb_mode,
            dmg_compat: false,
            compat_bg_ref: [0u32; 4],
            compat_obj_ref: [[0u32; 4]; 2],
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
            window_line: 0,
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
            dot_x: 0,
            scan_sprite_count: 0,
            scan_sprites: [SpriteData::default(); 10],
            bg_row: [0u8; SCREEN_WIDTH],
            sprite_occupied: [false; SCREEN_WIDTH],
            window_triggered: false,
        }
    }

    fn compare(&self) -> bool {
        self.line == self.compare_line
    }

    pub fn get_buffer(&self) -> &[u32; FRAMEBUFFER_LEN] {
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

    /// Recompute the colour slot for a DMG palette register (FF47/FF48/FF49), choosing the
    /// reference palette by mode: boot-ROM colours in DMG-compat, the configured DMG palette
    /// on real DMG, and nothing in native CGB mode.
    fn dmg_palette_colors(&self, palette: &Palette, compat_ref: &[u32; 4]) -> Option<[u32; 4]> {
        if self.dmg_compat {
            Some(Self::dmg_colors_from_palette(palette, compat_ref))
        } else if !self.cgb_mode {
            Some(Self::dmg_colors_from_palette(palette, &self.dmg_palette))
        } else {
            None
        }
    }

    /// Write one byte to a CGB palette-data register (FF69/FF6B): store it, recompute the
    /// affected colour, and auto-increment the index control register when its bit 7 is set.
    /// Returns the index that was written.
    fn write_cgb_palette(
        palette_data: &mut [u8; 64],
        colors: &mut [[u32; 4]; 8],
        ctrl: &mut u8,
        byte: u8,
    ) -> u8 {
        let index = *ctrl & 0x3F;
        palette_data[index as usize] = byte;
        Self::recompute_cgb_color(palette_data, colors, index);
        if *ctrl & 0x80 != 0 {
            *ctrl = (*ctrl & 0x80) | ((index + 1) & 0x3F);
        }
        index
    }

    fn color15_to_xrgb(c: u16) -> u32 {
        let r = (c & 0x1F) as u32;
        let g = ((c >> 5) & 0x1F) as u32;
        let b = ((c >> 10) & 0x1F) as u32;
        if COLOR_CORRECTION {
            // SameBoy-style correction: GBC LCD cross-channel mixing, normalised to 8-bit
            let r_out = (r * 26 + g * 4 + b * 2) * 255 / (32 * 31);
            let g_out = (g * 24 + b * 8) * 255 / (32 * 31);
            let b_out = (r * 6 + g * 4 + b * 22) * 255 / (32 * 31);
            (r_out << 16) | (g_out << 8) | b_out
        } else {
            let expand = |v: u32| (v << 3) | (v >> 2);
            (expand(r) << 16) | (expand(g) << 8) | expand(b)
        }
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

        offset + BYTES_PER_TILE_ROW * TILE_SIZE * (index as usize)
    }

    /// Combine the two bit-planes of a tile row into a 2-bit colour number for `bit_pos`.
    fn decode_pixel(byte_low: u8, byte_high: u8, bit_pos: u8) -> u8 {
        let high_bit = is_bit_set(bit_pos, byte_high as u16) as u8;
        let low_bit = is_bit_set(bit_pos, byte_low as u16) as u8;
        (high_bit << 1) | low_bit
    }

    // Decode one BG/window tile pixel. Returns (colour_number, XRGB8888 color, bg_priority).
    fn resolve_tile_pixel(
        &self,
        tilemap_index: usize,
        cell_x: usize,
        cell_y: usize,
    ) -> (u8, u32, bool) {
        let tile_id = self.vram[tilemap_index]; // bank 0
        let attr = if self.dmg_compat {
            0
        } else {
            self.vram[VRAM_BANK_SIZE + tilemap_index]
        };
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
        let bank_offset = tile_bank * VRAM_BANK_SIZE;

        let tileset_index = self.get_tileset_index(tile_id) + BYTES_PER_TILE_ROW * effective_cell_y;
        let byte_1 = self.vram[bank_offset + tileset_index];
        let byte_2 = self.vram[bank_offset + tileset_index + 1];

        let colour_number = Self::decode_pixel(byte_1, byte_2, bit_pos);

        (
            colour_number,
            self.bg_colors[palette_idx][colour_number as usize],
            bg_priority,
        )
    }

    /// Look up the BG/window pixel at absolute (`pixel_x`, `pixel_y`) within the tilemap
    /// selected by `use_map1`, returning (colour_number, XRGB8888 color, bg_priority).
    fn tilemap_pixel(&self, pixel_x: usize, pixel_y: usize, use_map1: bool) -> (u8, u32, bool) {
        let tilemap_offset = if use_map1 {
            TILEMAP1_OFFSET
        } else {
            TILEMAP0_OFFSET
        };
        let tilemap_y = (pixel_y / TILE_SIZE) % TILES_IN_A_TILEMAP_COL;
        let cell_y = pixel_y % TILE_SIZE;
        let tilemap_x = (pixel_x / TILE_SIZE) % TILES_IN_A_TILEMAP_ROW;
        let cell_x = pixel_x % TILE_SIZE;
        let tilemap_index = tilemap_offset + tilemap_y * TILES_IN_A_TILEMAP_ROW + tilemap_x;
        self.resolve_tile_pixel(tilemap_index, cell_x, cell_y)
    }

    /// Render one pixel (screen column `px`) of the current scanline into the framebuffer.
    /// Called once per T-cycle during Mode 3.  Uses whatever register state is current at
    /// that moment, giving correct mid-scanline effects for scroll / palette changes.
    fn render_dot(&mut self, px: usize) {
        let line = self.line as usize;
        let fb_index = line * SCREEN_WIDTH + px;

        // --- BG / Window ---
        let win_x_adj: usize = if self.window_x < 7 {
            0
        } else {
            (self.window_x - 7) as usize
        };
        let use_window = self.window_enabled && (self.window_y <= self.line) && px >= win_x_adj;

        let (colour_number, color, bg_priority) = if use_window {
            self.window_triggered = true;
            let win_px = px - win_x_adj;
            let win_line = self.window_line as usize;
            self.tilemap_pixel(win_px, win_line, self.window_map)
        } else if self.cgb_mode || self.bg_enabled {
            let line_to_draw = line.wrapping_add(self.scroll_y as usize) & 0xFF;
            let curr_pixel_x = self.scroll_x as usize + px;
            self.tilemap_pixel(curr_pixel_x, line_to_draw, self.bg_map)
        } else {
            // BG disabled in DMG mode: blank (colour 0, white)
            (0, self.bg_colors[0][0], false)
        };

        self.bg_row[px] = colour_number | ((bg_priority as u8) << 2);
        self.buffer[fb_index] = color;

        // --- Sprite compositing ---
        if self.obj_enabled {
            let sprite_height = if self.obj_size { 16u8 } else { 8u8 };
            let count = self.scan_sprite_count as usize;

            for i in 0..count {
                let spr = self.scan_sprites[i];
                // spr.x is the raw OAM X byte; screen left edge = spr.x - 8
                let spr_screen_left = spr.x as i16 - 8;
                let spr_col = px as i16 - spr_screen_left; // column within sprite (0 = leftmost)
                if !(0..8).contains(&spr_col) {
                    continue;
                }

                // A higher-priority sprite already drew here; no lower sprite can overwrite
                if self.sprite_occupied[px] {
                    break;
                }

                let flip_x = (spr.attr & 0x20) != 0;
                let flip_y = (spr.attr & 0x40) != 0;
                let z = (spr.attr & 0x80) != 0;
                let cgb_palette = if self.cgb_mode && !self.dmg_compat {
                    (spr.attr & 0x07) as usize
                } else {
                    ((spr.attr >> 4) & 0x01) as usize
                };
                let tile_vbank = if self.cgb_mode && !self.dmg_compat {
                    ((spr.attr >> 3) & 0x01) as usize
                } else {
                    0
                };

                let screen_y = spr.y.wrapping_sub(16);
                let mut sprite_row = self.line.wrapping_sub(screen_y);
                let mut tile = if self.obj_size {
                    spr.tile & 0xFE
                } else {
                    spr.tile
                };

                if flip_y {
                    sprite_row = sprite_height - sprite_row - 1;
                }
                if sprite_row >= 8 {
                    tile = tile.wrapping_add(1);
                    sprite_row -= 8;
                }

                let bank_offset = tile_vbank * VRAM_BANK_SIZE;
                let tile_offset = TILEDATA1_OFFSET
                    + BYTES_PER_TILE_ROW * TILE_SIZE * tile as usize
                    + sprite_row as usize * BYTES_PER_TILE_ROW;
                let byte_1 = self.vram[bank_offset + tile_offset];
                let byte_2 = self.vram[bank_offset + tile_offset + 1];

                // bit_pos: no flip → leftmost pixel = bit 7 (MSB); flip → leftmost = bit 0
                let bit_pos = if flip_x {
                    spr_col as u8
                } else {
                    7 - spr_col as u8
                };
                let spr_colour = Self::decode_pixel(byte_1, byte_2, bit_pos);

                if spr_colour == 0 {
                    continue; // transparent; let lower-priority sprite try
                }

                let bg_wins = if self.cgb_mode && !self.bg_master_priority {
                    false
                } else {
                    (self.bg_row[px] & 0x04 != 0) || (z && self.bg_row[px] & 0x03 != 0)
                };
                if bg_wins {
                    break; // BG wins; pixel is settled — lower sprites don't retry
                }

                self.sprite_occupied[px] = true;
                self.buffer[fb_index] = self.obj_colors[cgb_palette][spr_colour as usize];
                break;
            }
        }
    }

    /// Pack the LCDC register (0xFF40). Bit 0 is mode-dependent: BG/window master
    /// priority in CGB mode, BG enable in DMG mode.
    fn read_lcdc(&self) -> u8 {
        let bit0 = if self.cgb_mode {
            self.bg_master_priority
        } else {
            self.bg_enabled
        };
        (bit0 as u8)
            | ((self.obj_enabled as u8) << 1)
            | ((self.obj_size as u8) << 2)
            | ((self.bg_map as u8) << 3)
            | ((self.bg_tile as u8) << 4)
            | ((self.window_enabled as u8) << 5)
            | ((self.window_map as u8) << 6)
            | ((self.lcd_enabled as u8) << 7)
    }

    /// Unpack a write to the LCDC register (0xFF40), including the LCD enable/disable
    /// state transitions.
    fn write_lcdc(&mut self, byte: u8) {
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
            self.window_line = 0;
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
            self.window_line = 0;
            // lyc_flag intentionally NOT updated here — frozen at last value
            // stat_line = lyc_source only (mode sources gated by lcd_enabled)
            self.stat_line = self.compute_stat_line();
        }
    }

    /// Pack the STAT register (0xFF41). Bit 7 reads as 1; mode bits read 0 during the
    /// brief mode-0 startup window after LCD enable.
    fn read_stat(&self) -> u8 {
        let mode_bits = if self.lcd_startup_ticks > 0 {
            0
        } else {
            self.mode & 0x03
        };
        0x80 | mode_bits
            | ((self.compare_enabled as u8) << 6)
            | ((self.mode2_int_enabled as u8) << 5)
            | ((self.mode1_int_enabled as u8) << 4)
            | ((self.mode0_int_enabled as u8) << 3)
            | ((self.lyc_flag as u8) << 2)
    }

    /// Unpack a write to the STAT register (0xFF41), re-evaluating the STAT interrupt line.
    fn write_stat(&mut self, byte: u8) {
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

    /// Collect full sprite data for all sprites visible on the current scanline.
    fn collect_sprites_full(&self) -> ([SpriteData; 10], usize) {
        let mut sprites = [SpriteData::default(); 10];
        let mut count = 0usize;
        if !self.obj_enabled {
            return (sprites, 0);
        }
        let sprite_height: u8 = if self.obj_size { 16 } else { 8 };
        for i in 0..40usize {
            let base = i * 4;
            let y = self.oam[base];
            let x = self.oam[base + 1];
            let screen_y = y.wrapping_sub(16);
            if x < 168 && self.line.wrapping_sub(screen_y) < sprite_height {
                sprites[count] = SpriteData {
                    y,
                    x,
                    tile: self.oam[base + 2],
                    attr: self.oam[base + 3],
                };
                count += 1;
                if count >= 10 {
                    break;
                }
            }
        }
        (sprites, count)
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
                if self.modeclock >= MODE2_DOTS {
                    self.modeclock = 0;
                    self.mode = 3;
                    self.accessed_oam_row = 0xFF;
                    // Collect sprite data; extract X values for the timing penalty calculation
                    let (mut sprites, scount) = self.collect_sprites_full();
                    let xs: [u8; 10] = std::array::from_fn(|i| sprites[i].x);
                    self.mode3_extra = Self::compute_mode3_sprite_penalty(&xs[..scount]);
                    self.mode3_extra += (self.scroll_x & 7) as u16;
                    // DMG/compat: lower X wins; CGB: OAM-index order (already collected that way)
                    if !self.cgb_mode || self.dmg_compat {
                        sprites[..scount].sort_by_key(|s| s.x);
                    }
                    self.scan_sprites = sprites;
                    self.scan_sprite_count = scount as u8;
                    // Reset per-scanline pixel state
                    self.dot_x = 0;
                    self.bg_row = [0u8; SCREEN_WIDTH];
                    self.sprite_occupied = [false; SCREEN_WIDTH];
                    self.window_triggered = false;
                }
            }
            // scanline, vram read mode — render one pixel per T-cycle
            3 => {
                self.accessed_oam_row = 0xFF;
                if (self.dot_x as usize) < SCREEN_WIDTH {
                    self.render_dot(self.dot_x as usize);
                    self.dot_x += 1;
                }
                if self.modeclock >= MODE3_BASE_DOTS + self.mode3_extra {
                    self.modeclock = 0;
                    self.mode = 0;
                    self.hblank_flag = true;
                    // Flush any pixels that weren't rendered (shouldn't happen: 160 ≤ 172)
                    while (self.dot_x as usize) < SCREEN_WIDTH {
                        self.render_dot(self.dot_x as usize);
                        self.dot_x += 1;
                    }
                    // WLY increments only when at least one window pixel was output this scanline.
                    // (WX=167 parks the window off-screen; real hardware does not advance WLY.)
                    if self.window_triggered {
                        self.window_line += 1;
                    }
                }
            }
            // hblank
            0 => {
                self.accessed_oam_row = 0xFF;
                if self.modeclock >= MODE0_BASE_DOTS - self.mode3_extra {
                    self.modeclock = 0;
                    self.line += 1;
                    self.lyc_flag = self.line == self.compare_line;

                    if self.line == VBLANK_START_LINE {
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
                if self.modeclock >= SCANLINE_DOTS {
                    self.modeclock = 0;
                    self.line += 1;

                    if self.line > LAST_LINE {
                        self.mode = 2;
                        self.accessed_oam_row = 0;
                        self.line = 0;
                        self.window_line = 0;
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

    // Verify per-dot rendering: a scroll_x change mid-Mode-3 affects only the remaining pixels.
    // With the old batch renderer the whole line would use the final scroll_x value.
    #[test]
    fn test_mid_scanline_scroll_x() {
        let mut gpu = GPU::new(false);

        // Standard DMG palette: colour 0→off, 1→light, 2→dark, 3→on
        gpu.write_byte(0xFF47, 0xE4);

        // Tile 0 (TILEDATA1, offset 0): all pixels = colour 1
        // byte_1=0xFF, byte_2=0x00 → colour_number = (0<<1)|1 = 1
        for row in 0..8usize {
            gpu.vram[row * 2] = 0xFF;
            gpu.vram[row * 2 + 1] = 0x00;
        }
        // Tile 1 (offset 16): all pixels = colour 3
        // byte_1=0xFF, byte_2=0xFF → colour_number = (1<<1)|1 = 3
        for row in 0..8usize {
            gpu.vram[16 + row * 2] = 0xFF;
            gpu.vram[16 + row * 2 + 1] = 0xFF;
        }
        // Tilemap row 0 (TILEMAP0): col 0 = tile 0, cols 1..31 = tile 1
        gpu.vram[TILEMAP0_OFFSET] = 0;
        for col in 1..32usize {
            gpu.vram[TILEMAP0_OFFSET + col] = 1;
        }

        // Put GPU directly into Mode 3 start (line 0, no sprites, no window)
        gpu.lcd_enabled = true;
        gpu.bg_enabled = true;
        gpu.bg_tile = true; // TILEDATA1: tiles at VRAM offset 0
        gpu.bg_map = false; // TILEMAP0
        gpu.obj_enabled = false;
        gpu.window_enabled = false;
        gpu.scroll_x = 0;
        gpu.scroll_y = 0;
        gpu.line = 0;
        gpu.mode = 3;
        gpu.modeclock = 0;
        gpu.mode3_extra = 0;
        gpu.dot_x = 0;
        gpu.bg_row = [0u8; 160];
        gpu.sprite_occupied = [false; 160];
        gpu.scan_sprite_count = 0;

        // Render the first 8 pixels with scroll_x=0 → tile 0 → colour 1
        for _ in 0..8 {
            gpu.step(1);
        }

        // Change scroll_x mid-scanline: pixels 8..159 now map into tile 1 territory
        gpu.scroll_x = 8;

        // Drive Mode 3 to completion (172 T-cycles total; 8 already consumed)
        for _ in 0..164 {
            gpu.step(1);
        }

        let color_1 = gpu.bg_colors[0][1]; // tile 0
        let color_3 = gpu.bg_colors[0][3]; // tile 1
        assert_ne!(
            color_1, color_3,
            "palette colours must be distinct for this test to be meaningful"
        );

        // Pixels 0-7 were emitted before the scroll change → tile 0
        for px in 0..8usize {
            assert_eq!(
                gpu.buffer[px], color_1,
                "pixel {px}: expected tile-0 colour"
            );
        }
        // Pixels 8-159 were emitted after the scroll change → tile 1
        for px in 8..160usize {
            assert_eq!(
                gpu.buffer[px], color_3,
                "pixel {px}: expected tile-1 colour"
            );
        }
    }
}
