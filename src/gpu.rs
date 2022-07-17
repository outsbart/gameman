use crate::cpu::is_bit_set;
use std::iter;

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
}

#[derive(Clone, Copy)]
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

struct SpriteOptions {
    z: bool,       // 0 = above background, 1 = below background (unless colour is 0)
    flip_y: bool,  // 1 = flipped vertically
    flip_x: bool,  // 1 = flipped horizontally
    palette: bool, // 0 meanse use object palette 0, 1 means use object palette 1
}

impl SpriteOptions {
    pub fn new() -> Self {
        SpriteOptions {
            z: false,
            flip_y: false,
            flip_x: false,
            palette: false,
        }
    }

    pub fn update(&mut self, value: u8) {
        self.palette = (value & 0x10) != 0;
        self.flip_x = (value & 0x20) != 0;
        self.flip_y = (value & 0x40) != 0;
        self.z = (value & 0x80) != 0;
    }

    pub fn byte(&self) -> u8 {
        (if self.palette { 0x10 } else { 0 })
            | (if self.flip_x { 0x20 } else { 0 })
            | (if self.flip_y { 0x40 } else { 0 })
            | (if self.z { 0x80 } else { 0 })
    }
}

struct Sprite {
    y: u8,           // y coordinate of top left corner, minus 16
    x: u8,           // x coordinate of top left corner, minus 8
    tile_number: u8, // which tile to use
    options: SpriteOptions,
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y: 0,
            x: 0,
            tile_number: 0,
            options: SpriteOptions::new(),
        }
    }

    pub fn update(&mut self, field_num: u8, value: u8) {
        match field_num {
            0 => self.y = value.wrapping_sub(16),
            1 => self.x = value.wrapping_sub(8),
            2 => self.tile_number = value,
            3 => self.options.update(value),
            _ => {
                panic!("Unhandled sprite field update")
            }
        }
    }

    pub fn get(&self, field_num: u8) -> u8 {
        match field_num {
            0 => self.y.wrapping_add(16),
            1 => self.x.wrapping_add(8),
            2 => self.tile_number,
            3 => self.options.byte(),
            _ => {
                panic!("Unhandled sprite field read")
            }
        }
    }
}

pub struct GPU {
    vram: [u8; 8192],
    sprites: Vec<Sprite>,    // todo: make it an array of 40
    buffer: [u8; 160 * 144], // every pixel can have 4 values (4 shades of grey)

    modeclock: u16,
    mode: u8,
    line: u8,

    bg_enabled: bool,     // draw bg?
    obj_enabled: bool,    // draw sprites?
    obj_size: bool,       // sprite is tall 16 or 8 pixel?
    bg_map: bool,         // which tilemap to use for the bg
    bg_tile: bool,        // tiles data to use for both bg and window
    window_enabled: bool, // draw window?
    window_map: bool,     // which tilemap use for the window?
    lcd_enabled: bool,

    compare_enabled: bool, // stat reg. Should compare with compare line?
    compare_line: u8,      // when line == compare_line an interrupt is triggered

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
        let sprite_num = addr >> 2;
        if sprite_num > 39 {
            panic!("Tried to access sprite number {}", sprite_num);
        }

        let property = (addr & 3) as u8;
        self.sprites[sprite_num as usize].get(property)
    }
    fn write_oam(&mut self, addr: u16, byte: u8) {
        let sprite_num = addr >> 2;
        if sprite_num > 39 {
            panic!("Tried to update sprite number {}", sprite_num);
        }

        let property = (addr & 3) as u8;
        self.sprites[sprite_num as usize].update(property, byte);
    }
    fn read_vram(&mut self, addr: u16) -> u8 {
        self.vram[addr as usize]
    }
    fn write_vram(&mut self, addr: u16, byte: u8) {
        self.vram[addr as usize] = byte
    }
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF40 => {
                (if self.bg_enabled { 0x01 } else { 0 })
                    | (if self.obj_enabled { 0x02 } else { 0 })
                    | (if self.obj_size { 0x04 } else { 0 })
                    | (if self.bg_map { 0x08 } else { 0 })
                    | (if self.bg_tile { 0x10 } else { 0 })
                    | (if self.window_enabled { 0x20 } else { 0 })
                    | (if self.window_map { 0x40 } else { 0 })
                    | (if self.lcd_enabled { 0x80 } else { 0 })
            }
            0xFF41 => {
                (if self.compare_enabled { 0x20 } else { 0 })
                    | (if self.compare() { 0x04 } else { 0 })
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
            _ => 0,
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr {
            0xFF40 => {
                // LCD Control
                self.bg_enabled = (byte & 0x01) != 0;
                self.obj_enabled = (byte & 0x02) != 0;
                self.obj_size = (byte & 0x04) != 0;
                self.bg_map = (byte & 0x08) != 0;
                self.bg_tile = (byte & 0x10) != 0;
                self.window_enabled = (byte & 0x20) != 0;
                self.window_map = (byte & 0x40) != 0;
                self.lcd_enabled = (byte & 0x80) != 0;
            }
            0xFF41 => {
                self.compare_enabled = (byte & 0x40) != 0;
            }
            0xFF42 => {
                self.scroll_y = byte;
            }
            0xFF43 => {
                self.scroll_x = byte;
            }
            0xFF44 => {
                self.line = 0;
                println!("line reset");
            }
            0xFF45 => {
                self.compare_line = byte;
            }
            0xFF46 => {
                // DMA transfer, handled from outside
            }
            0xFF47 => {
                self.bg_palette.update(byte);
            }
            0xFF48 => {
                self.obj_palette_0.update(byte);
            }
            0xFF49 => {
                self.obj_palette_1.update(byte);
            }
            0xFF4A => {
                self.window_y = byte;
            }
            0xFF4B => {
                self.window_x = byte;
            }
            _ => {}
        }
    }
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            vram: [0; 8192],
            sprites: iter::repeat_with(Sprite::new).take(40).collect(),
            buffer: [0; 160 * 144],
            modeclock: 0,
            mode: 2,
            line: 0,
            bg_enabled: false,
            obj_enabled: false,
            obj_size: false,
            bg_map: false,
            bg_tile: false,
            window_enabled: false,
            window_map: false,
            lcd_enabled: false,
            compare_enabled: false,
            compare_line: 0,
            scroll_x: 0,
            scroll_y: 0,
            bg_palette: Palette::new(),
            obj_palette_0: Palette::new(),
            obj_palette_1: Palette::new(),
            window_x: 0,
            window_y: 0,
        }
    }

    fn compare(&self) -> bool {
        self.line == self.compare_line
    }

    pub fn get_buffer(&self) -> &[u8; 160 * 144] {
        &self.buffer
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

    // draws a line on the buffer
    pub fn render_scan_to_buffer(&mut self) {
        let line_to_draw: usize = self.line.wrapping_add(self.scroll_y) as usize;

        // save colour numbers being rendered before palette application. 0 is transparent
        let mut rendering_row = [0u8; 160];

        // background
        if self.bg_enabled {
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
                    tilemap_offset + (tilemap_y * TILES_IN_A_TILEMAP_ROW + tilemap_x) as usize;

                let pos = self.vram[tilemap_index];

                // find out the row in the tile data
                let tileset_index: usize = self.get_tileset_index(pos) + 2 * cell_y as usize;

                // a tile pixel line is encoded in two consecutive bytes
                let byte_1 = self.vram[tileset_index];
                let byte_2 = self.vram[tileset_index + 1];

                // get the pixel colour from the line
                let high_bit: u8 = is_bit_set(7 - cell_x as u8, byte_2 as u16) as u8;
                let low_bit: u8 = is_bit_set(7 - cell_x as u8, byte_1 as u16) as u8;
                let colour_number = (high_bit << 1) + low_bit;
                let palette_colour = self.bg_palette.get(colour_number);

                rendering_row[row_pixel] = colour_number;

                let index: usize =
                    (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + row_pixel;
                self.buffer[index] = palette_colour as u8;
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

            // the row of the cell in the window tilemap
            let tilemap_y: usize = (window_line / TILE_SIZE) % TILES_IN_A_TILEMAP_COL;

            // the row of the pixel in the cell
            let cell_y: usize = window_line % TILE_SIZE;

            #[allow(clippy::needless_range_loop)]
            for pixel in (window_x as usize)..TILES_IN_A_SCREEN_ROW * TILE_SIZE {
                let mut curr_pixel_x = (pixel as u8).wrapping_add(self.scroll_x);
                if curr_pixel_x >= window_x {
                    curr_pixel_x = pixel as u8 - window_x;
                }

                // the col of the cell in the tilemap
                let tilemap_x: usize = (curr_pixel_x as usize / TILE_SIZE) % TILES_IN_A_TILEMAP_ROW;

                // the col of the pixel in the cell
                let cell_x: usize = curr_pixel_x as usize % TILE_SIZE;

                // find the tile in the vram
                let tilemap_index =
                    tilemap_offset + (tilemap_y * TILES_IN_A_TILEMAP_ROW + tilemap_x) as usize;

                let pos = self.vram[tilemap_index];

                // find out the row in the tile data
                let tileset_index: usize = self.get_tileset_index(pos) + 2 * cell_y as usize;

                // a tile pixel line is encoded in two consecutive bytes
                let byte_1 = self.vram[tileset_index];
                let byte_2 = self.vram[tileset_index + 1];

                // get the pixel colour from the line
                let high_bit: u8 = is_bit_set(7 - cell_x as u8, byte_2 as u16) as u8;
                let low_bit: u8 = is_bit_set(7 - cell_x as u8, byte_1 as u16) as u8;
                let colour_number = (high_bit << 1) + low_bit;
                let palette_colour = self.bg_palette.get(colour_number);

                rendering_row[pixel] = colour_number;

                let index: usize = (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + pixel;
                self.buffer[index] = palette_colour as u8;
            }
        }

        // sprites
        if self.obj_enabled {
            let sprite_height: u8 = if self.obj_size { 16 } else { 8 };

            for sprite_num in 0..40 {
                let sprite = &self.sprites[sprite_num];

                // not insersecting with scanline, dont draw
                if self.line.wrapping_sub(sprite.y) >= sprite_height {
                    continue;
                }

                let mut pos = sprite.tile_number;

                // handle upside down
                let mut sprite_pixel_row = if sprite.options.flip_y {
                    sprite_height - self.line.wrapping_sub(sprite.y) - 1
                } else {
                    self.line.wrapping_sub(sprite.y)
                };

                // go to next tile if we have to render 2nd part of the 16pixel sprite
                if sprite_pixel_row >= 8 {
                    pos = pos.wrapping_add(1);
                    sprite_pixel_row -= 8;
                }

                // sprites always use tiledata1
                let tile_in_tileset: usize = TILEDATA1_OFFSET
                    + (2 * 8 * pos as usize + sprite_pixel_row as usize * 2) as usize;

                // a tile pixel line is encoded in two consecutive bytes
                let byte_1 = self.vram[tile_in_tileset];
                let byte_2 = self.vram[tile_in_tileset + 1];

                for pixel in 0..8u8 {
                    let ix = if sprite.options.flip_x {
                        pixel
                    } else {
                        7 - pixel
                    };

                    let curr_x = sprite.x.wrapping_add(7 - pixel);

                    // out of the line, don't draw
                    if curr_x >= 160 {
                        continue;
                    }

                    let high_bit: u8 = is_bit_set(7 - ix, byte_2 as u16) as u8;
                    let low_bit: u8 = is_bit_set(7 - ix, byte_1 as u16) as u8;

                    let colour_number = (high_bit << 1) + low_bit;

                    // transparent, don't draw
                    if colour_number == 0 {
                        continue;
                    }

                    // bg pixel wins over sprite, don't draw
                    if sprite.options.z && (rendering_row[curr_x as usize] != 0) {
                        continue;
                    }

                    let palette = if sprite.options.palette {
                        &self.obj_palette_1
                    } else {
                        &self.obj_palette_0
                    };
                    let colour = palette.get(colour_number);
                    let index: usize =
                        (self.line as usize * TILES_IN_A_SCREEN_ROW * TILE_SIZE) + curr_x as usize;
                    self.buffer[index] = colour as u8;
                }
            }
        }
    }

    // returns true if compare stat interrupt should raise
    fn check_compare_int(&self) -> bool {
        self.compare_enabled && self.compare()
    }

    // go forward based on the cpu's last operation clocks
    pub fn step(&mut self, t: u8) -> (bool, bool) {
        self.modeclock += t as u16;

        let mut vblank_interrupt: bool = false;
        let mut compare_interrupt: bool = false;

        // todo: implement it as a state machine?
        match self.mode {
            // scanline, oam read mode
            2 => {
                if self.modeclock >= 80 {
                    self.modeclock = 0;
                    self.mode = 3;
                }
            }
            // scanline, vram read mode
            3 => {
                if self.modeclock >= 172 {
                    // enter hblank mode
                    self.modeclock = 0;
                    self.mode = 0;

                    self.render_scan_to_buffer();
                }
            }
            // hblank
            0 => {
                if self.modeclock >= 204 {
                    self.modeclock = 0;
                    self.line += 1;

                    if self.line == 144 {
                        // enter vblank mode
                        self.mode = 1;
                        vblank_interrupt = true;
                    } else {
                        self.mode = 2;
                    }

                    compare_interrupt = self.check_compare_int();
                }
            }
            // vblank (10 lines)
            1 => {
                if self.modeclock >= 456 {
                    self.modeclock = 0;
                    self.line += 1;

                    // restart
                    if self.line > 153 {
                        self.mode = 2;
                        self.line = 0;
                    }

                    compare_interrupt = self.check_compare_int();
                }
            }
            _ => panic!("Sorry what?"),
        }

        (vblank_interrupt, compare_interrupt)
    }
}

impl Default for GPU {
    fn default() -> Self {
        GPU::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // test scroll_y write and read access, as well as the default value
    #[test]
    fn test_scroll_y() {
        let mut gpu = GPU::new();

        assert_eq!(gpu.scroll_y, 0);

        gpu.write_byte(0xFF42, 1);

        assert_eq!(gpu.scroll_y, 1);
        assert_eq!(gpu.read_byte(0xFF42), 1);
    }

    // test scroll_x write and read access, as well as the default value
    #[test]
    fn test_scroll_x() {
        let mut gpu = GPU::new();

        assert_eq!(gpu.scroll_x, 0);

        gpu.write_byte(0xFF43, 1);

        assert_eq!(gpu.scroll_x, 1);
        assert_eq!(gpu.read_byte(0xFF43), 1);
    }

    // test palette write and read access, as well as the default value
    #[test]
    fn test_bg_palette() {
        let mut gpu = GPU::new();

        // default value
        assert_eq!(gpu.bg_palette.byte, 0xFF);

        gpu.write_byte(0xFF47, 1);

        assert_eq!(gpu.bg_palette.byte, 1);
    }

    // test obj palette 0 write and read access, as well as the default value
    #[test]
    fn test_obj_palette_0() {
        let mut gpu = GPU::new();

        // default value
        assert_eq!(gpu.obj_palette_0.byte, 0xFF);

        gpu.write_byte(0xFF48, 1);

        assert_eq!(gpu.obj_palette_0.byte, 1);
    }

    // test palette write and read access, as well as the default value
    #[test]
    fn test_obj_palette_1() {
        let mut gpu = GPU::new();

        // default value
        assert_eq!(gpu.obj_palette_1.byte, 0xFF);

        gpu.write_byte(0xFF49, 1);

        assert_eq!(gpu.obj_palette_1.byte, 1);
    }

    #[test]
    fn test_window_x_y() {
        let mut gpu = GPU::new();

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
        let mut gpu = GPU::new();

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
        let mut gpu = GPU::new();

        assert_eq!(gpu.line, 0);
        gpu.write_byte(0xFF44, 1);
        // no write access
        assert_eq!(gpu.line, 0);

        gpu.line = 15;
        assert_eq!(gpu.read_byte(0xFF44), 15);
    }

    // test sprite write and read in the oam area 0xFE00-0xFE9F
    #[test]
    fn test_sprite() {
        let mut gpu = GPU::new();

        // should update first sprite's first property
        gpu.write_oam(0, 18);
        assert_eq!(gpu.sprites[0].y, 2);
        assert_eq!(gpu.read_oam(0), 18);

        // should update first sprite's 2nd property
        gpu.write_oam(1, 14);
        assert_eq!(gpu.sprites[0].x, 6);
        assert_eq!(gpu.read_oam(1), 14);

        // should update first sprite's 3rd property
        assert_eq!(gpu.sprites[0].tile_number, 0);
        gpu.write_oam(2, 4);
        assert_eq!(gpu.sprites[0].tile_number, 4);
        assert_eq!(gpu.read_oam(2), 4);

        // should update first sprite's options z
        assert!(!gpu.sprites[0].options.z);
        gpu.write_oam(3, 0b10000000);
        assert!(gpu.sprites[0].options.z);
        assert_eq!(gpu.read_oam(3), 0b10000000);

        // should update first sprite's options flip_y
        assert!(!gpu.sprites[0].options.flip_y);
        gpu.write_oam(3, 0b01000000);
        assert!(gpu.sprites[0].options.flip_y);
        assert_eq!(gpu.read_oam(3), 0b01000000);

        // should update first sprite's options flip_x
        assert!(!gpu.sprites[0].options.flip_x);
        gpu.write_oam(3, 0b00100000);
        assert!(gpu.sprites[0].options.flip_x);
        assert_eq!(gpu.read_oam(3), 0b00100000);

        // should update first sprite's options flip_x
        assert!(!gpu.sprites[0].options.palette);
        gpu.write_oam(3, 0b00010000);
        assert!(gpu.sprites[0].options.palette);
        assert_eq!(gpu.read_oam(3), 0b00010000);

        // should update sprite 40's options flip_x
        assert!(!gpu.sprites[39].options.palette);
        gpu.write_oam(159, 0b00010000);
        assert!(gpu.sprites[39].options.palette);
        assert_eq!(gpu.read_oam(3), 0b00010000);
    }
}
