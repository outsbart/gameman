use crate::cpu::is_bit_set;
use std::iter;

/// Expose the memories of the GPU
pub trait GPUMemoriesAccess {
    fn read_oam(&mut self, addr: u16) -> u8;
    fn write_oam(&mut self, addr: u16, byte: u8);
    fn update_sprite(&mut self, addr: u16, byte: u8);
    fn read_vram(&mut self, addr: u16) -> u8;
    fn write_vram(&mut self, addr: u16, byte: u8);
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);
}

#[repr(u8)]
pub enum Color {
    Off = 0,
    Light = 1,
    Dark = 2,
    On = 3,
}

impl Color {
    #[inline]
    pub fn from_u8(value: u8) -> Color {
        use self::Color::*;
        match value {
            1 => Light,
            2 => Dark,
            3 => On,
            _ => Off,
        }
    }
}

impl Into<u8> for Color {
    fn into(self) -> u8 {
        match self {
            Color::Off => 0,
            Color::Light => 1,
            Color::Dark => 2,
            Color::On => 3,
        }
    }
}

struct SpriteOptions {
    z: bool, // 0 = above background, 1 = below background (unless colour is 0)
    flip_y: bool, // 1 = flipped vertically
    flip_x: bool, // 1 = flipped horizontally
    palette: bool, // 0 meanse use object palette 0, 1 means use object palette 1
}

impl SpriteOptions {
    pub fn new() -> Self {
        SpriteOptions {
            z: false,
            flip_y: false,
            flip_x: false,
            palette: false
        }
    }

    pub fn update(&mut self, value: u8) {
        self.palette = if (value & 0x10) != 0 { true } else { false };
        self.flip_x = if (value & 0x20) != 0 { true } else { false };
        self.flip_y = if (value & 0x40) != 0 { true } else { false };
        self.z = if (value & 0x80) != 0 { true } else { false };
    }
}

struct Sprite {
    y: u8, // y coordinate of top left corner, minus 16
    x: u8, // x coordinate of top left corner, minus 8
    tile_number: u8, // which tile to use
    options: SpriteOptions
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y: 0,
            x: 0,
            tile_number: 0,
            options: SpriteOptions::new()
        }
    }

    pub fn update(&mut self, field: u8, value: u8) {
        match field {
            0 => { self.y = value.wrapping_sub(16) }
            1 => { self.x = value.wrapping_sub(8) }
            2 => { self.tile_number = value }
            3 => { self.options.update(value) }
            _ => { panic!("Unhandled sprite field update")}
        }
    }
}


pub struct GPU {
    vram: [u8; 8192],
    oam: [u8; 256],
    sprites: Vec<Sprite>, // todo: make it an array of 40
    buffer: [u8; 160 * 144], // every pixel can have 4 values (4 shades of grey)

    modeclock: u16,
    mode: u8,
    line: u8,

    bg_enabled: bool,
    obj_enabled: bool,
    bg_map: bool,
    bg_tile: bool,
    lcd_enabled: bool,

    scroll_x: u8,
    scroll_y: u8,
    palette: u8,
    obj_palette_0: u8,
    obj_palette_1: u8,
}

impl GPUMemoriesAccess for GPU {
    fn read_oam(&mut self, addr: u16) -> u8 {
        self.oam[addr as usize]
    }
    fn write_oam(&mut self, addr: u16, byte: u8) {
        self.oam[addr as usize] = byte;
    }

    fn update_sprite(&mut self, addr: u16, byte: u8) {
        self.update_sprite(addr, byte);
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
                    | (if self.bg_map { 0x08 } else { 0 })
                    | (if self.bg_tile { 0x10 } else { 0 })
                    | (if self.lcd_enabled { 0x80 } else { 0 })
            },
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.line,
            _ => 0,
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr {
            0xFF40 => {
                self.bg_enabled = if (byte & 0x01) != 0 { true } else { false };
                self.obj_enabled = if (byte & 0x02) != 0 { true } else { false };
                self.bg_map = if (byte & 0x08) != 0 { true } else { false };
                self.bg_tile = if (byte & 0x10) != 0 { true } else { false };
                self.lcd_enabled = if (byte & 0x80) != 0 { true } else { false };
            }
            0xFF42 => {
                self.scroll_y = byte;
            }
            0xFF43 => {
                self.scroll_x = byte;
            }
            0xFF47 => {
                self.palette = byte;
            }
            0xFF48 => {
                self.obj_palette_0 = byte;
            }
            0xFF49 => {
                self.obj_palette_1 = byte;
            }
            _ => {}
        }
    }
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            vram: [0; 8192],
            oam: [0; 256],
            sprites: iter::repeat_with(|| Sprite::new()).take(40).collect(),
            buffer: [0; 160 * 144],
            modeclock: 0,
            mode: 2,
            line: 0,
            bg_enabled: false,
            obj_enabled: false,
            bg_map: false,
            bg_tile: false,
            lcd_enabled: false,
            scroll_x: 0,
            scroll_y: 0,
            palette: 0,
            obj_palette_0: 0,
            obj_palette_1: 0,
        }
    }

    pub fn update_sprite(&mut self, address: u16, value: u8) {
        let sprite_num = address >> 2;
        let property = (address & 3) as u8;
        if sprite_num < 40 {
            self.sprites[sprite_num as usize].update(property, value);
        }
    }

    pub fn get_buffer(&self) -> &[u8; 160 * 144] {
        return &self.buffer;
    }

    // draws a line on the buffer
    pub fn render_scan_to_buffer(&mut self) {
        // todo: reuse some calculations
        let (tiles_in_a_tilemap_row, tiles_in_a_screen_row, tile_size) = (32, 20, 8);
        let line_to_draw: usize = (self.line + self.scroll_y) as usize;
        let tilemap_row: usize = line_to_draw / tile_size; //todo: go back on top if line > 256
        let pixel_row: usize = line_to_draw % tile_size;

        let tilemap0_offset = 0x9800 - 0x8000;

        // background
        if self.bg_enabled {

            // a row is 20 tiles
            for tile in 0..tiles_in_a_screen_row {
                // todo: right now only draws the first 20 tiles from the left, use scroll X
                let tilemap_index =
                    tilemap0_offset + (tilemap_row * tiles_in_a_tilemap_row + tile) as usize;
                let pos = self.vram[tilemap_index];

                let tile_in_tileset: usize =
                    (2 * tile_size * (pos as usize) + (pixel_row as usize) * 2) as usize;

                // a tile pixel line is encoded in two consecutive bytes
                let byte_1 = self.vram[tile_in_tileset];
                let byte_2 = self.vram[tile_in_tileset + 1];

                for pixel in 0..8u8 {
                    let ix = 7 - pixel;
                    let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                    let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

                    let color: u8 = (high_bit << 1) + low_bit;
                    let index: usize = (self.line as usize * tiles_in_a_screen_row * tile_size)
                        + (tile as usize) * tile_size
                        + pixel as usize;

                    self.buffer[index] = color;
                }
            }

        }

        // sprites
        if self.obj_enabled {
            let sprite_size = 8; // todo: allow 16pixel sprites

            for sprite_num in 0..40 {
                let sprite = &self.sprites[sprite_num];

                // is it along scanline?
                if (sprite.y <= self.line) && (sprite.y + sprite_size > self.line) {
                    let sprite_pixel_row = self.line - sprite.y;

                    let pos = sprite.tile_number;
                    let tile_in_tileset: usize = (2 * sprite_size as usize * pos as usize * sprite_pixel_row as usize * 2) as usize;

                    // a tile pixel line is encoded in two consecutive bytes
                    let byte_1 = self.vram[tile_in_tileset];
                    let byte_2 = self.vram[tile_in_tileset + 1];

                    for pixel in 0..8u8 {
                        // check if it is in the screen. Is it within first 160 pixels?
                        // todo: use scroll_x instead of first 160...
                        if (sprite.x + pixel >= 0) && (sprite.x < 160) {
                            let ix = 7 - pixel;
                            let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                            let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

                            let color: u8 = (high_bit << 1) + low_bit;
                            let index: usize = (self.line as usize * tiles_in_a_screen_row * tile_size)
                                + sprite.x as usize + pixel as usize;

                            self.buffer[index] = color;
                        }
                    }
                }
            }
        }


    }

    // go forward based on the cpu's last operation clocks
    pub fn step(&mut self, t: u8) -> bool {
        self.modeclock += t as u16;

        let mut vblank_interrupt: bool = false;

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

                    if self.line == 143 {
                        // enter vblank mode
                        self.mode = 1;
                    } else {
                        self.mode = 2;
                    }
                }
            }
            // vblank (10 lines)
            1 => {
                if self.modeclock >= 456 {
                    self.modeclock = 0;
                    self.line += 1;

                    // restart
                    if self.line > 153 {
                        vblank_interrupt = true;

                        self.mode = 2;
                        self.line = 0;
                    }
                }
            }
            _ => panic!("Sorry what?"),
        }

        vblank_interrupt
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
    fn test_palette() {
        let mut gpu = GPU::new();

        // default value
        assert_eq!(gpu.palette, 0);

        gpu.write_byte(0xFF47, 1);

        assert_eq!(gpu.palette, 1);
        // no read access
        assert_eq!(gpu.read_byte(0xFF47), 0);
    }

    // test control write and read access, as well as the default value
    #[test]
    fn test_control() {
        let mut gpu = GPU::new();

        assert_eq!(gpu.bg_enabled, false);
        assert_eq!(gpu.obj_enabled, false);
        assert_eq!(gpu.bg_map, false);
        assert_eq!(gpu.bg_tile, false);
        assert_eq!(gpu.lcd_enabled, false);

        gpu.write_byte(0xFF40, 1);
        assert_eq!(gpu.bg_enabled, true);
        assert_eq!(gpu.read_byte(0xFF40), 1);

        gpu.write_byte(0xFF40, 0x02);
        assert_eq!(gpu.obj_enabled, true);
        assert_eq!(gpu.read_byte(0xFF40), 0x02);

        gpu.write_byte(0xFF40, 0x08);
        assert_eq!(gpu.bg_map, true);
        assert_eq!(gpu.read_byte(0xFF40), 0x08);

        gpu.write_byte(0xFF40, 0x10);
        assert_eq!(gpu.bg_tile, true);
        assert_eq!(gpu.read_byte(0xFF40), 0x10);

        gpu.write_byte(0xFF40, 0x80);
        assert_eq!(gpu.lcd_enabled, true);
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
        assert_eq!(gpu.sprites[0].y, -16);
        gpu.update_sprite(0, 18);
        assert_eq!(gpu.sprites[0].y, 2);

        // should update first sprite's 2nd property
        assert_eq!(gpu.sprites[0].x, -8);
        gpu.update_sprite(1, 14);
        assert_eq!(gpu.sprites[0].x, 6);

        // should update first sprite's 3rd property
        assert_eq!(gpu.sprites[0].tile_number, 0);
        gpu.update_sprite(2, 4);
        assert_eq!(gpu.sprites[0].tile_number, 4);

        // should update first sprite's options z
        assert_eq!(gpu.sprites[0].options.z, false);
        gpu.update_sprite(3, 0b10000000);
        assert_eq!(gpu.sprites[0].options.z, true);

        // should update first sprite's options flip_y
        assert_eq!(gpu.sprites[0].options.flip_y, false);
        gpu.update_sprite(3, 0b01000000);
        assert_eq!(gpu.sprites[0].options.flip_y, true);

        // should update first sprite's options flip_x
        assert_eq!(gpu.sprites[0].options.flip_x, false);
        gpu.update_sprite(3, 0b00100000);
        assert_eq!(gpu.sprites[0].options.flip_x, true);

        // should update first sprite's options flip_x
        assert_eq!(gpu.sprites[0].options.palette, false);
        gpu.update_sprite(3, 0b00010000);
        assert_eq!(gpu.sprites[0].options.palette, true);

        // should update sprite 40's options flip_x
        assert_eq!(gpu.sprites[39].options.palette, false);
        gpu.update_sprite(159, 0b00010000);
        assert_eq!(gpu.sprites[39].options.palette, true);
    }
}
