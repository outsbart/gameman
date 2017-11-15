use cpu::is_bit_set;

/// Expose the memories of the GPU
pub trait GPUMemoriesAccess {
    fn read_oam(&mut self, addr: u16) -> u8;
    fn write_oam(&mut self, addr: u16, byte: u8);
    fn read_vram(&mut self, addr: u16) -> u8;
    fn write_vram(&mut self, addr: u16, byte: u8);
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);
}

pub struct GPU {
    vram: [u8; 8192],
    oam: [u8; 256],
    buffer: [u8; 160 * 144],  // every pixel can have 4 values (4 shades of grey)

    modeclock: u16,
    mode: u8,
    line: u8,

    control: u8,
    scroll_x: u8,
    scroll_y: u8,
    palette: u8
}

impl GPUMemoriesAccess for GPU {
    fn read_oam(&mut self, addr: u16) -> u8 { self.oam[addr as usize] }
    fn write_oam(&mut self, addr: u16, byte: u8) { self.oam[addr as usize] = byte }
    fn read_vram(&mut self, addr: u16) -> u8 { self.vram[addr as usize] }
    fn write_vram(&mut self, addr: u16, byte: u8) { self.vram[addr as usize] = byte }
    fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0xFF40 => { self.control }
            0xFF42 => { self.scroll_y }
            0xFF43 => { self.scroll_x }
            0xFF44 => { self.line }
            _ => { 0 }
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr {
            0xFF40 => { self.control = byte; }
            0xFF42 => { self.scroll_y = byte; }
            0xFF43 => { self.scroll_x = byte; }
            0xFF47 => { self.palette = byte; }
            _ => {}
        }
    }
}

impl GPU {
    pub fn new() -> Self {
        GPU { vram: [0; 8192], oam: [0; 256], buffer: [0; 160 * 144], modeclock: 0, mode: 2, line: 0, scroll_x: 0, scroll_y: 0, palette: 0, control: 0 }
    }

    pub fn get_buffer(&self) -> &[u8; 160*144] {
        return &self.buffer;
    }

    pub fn render_scan_to_buffer(&mut self) {
        let tilemap_row: usize = (self.line + self.scroll_y) as usize / 8;  //todo: go back on top if line > 256
        let pixel_row = self.line % 8;
        let tilemap0_offset = 0x9800 - 0x8000;

        let scroll_x = 0u8;
        let scroll_y = 0u8;

        for tile in 0..20 {  // todo: right now only draws the first 20 tiles from the left
            let pos = self.vram[tilemap0_offset + (tilemap_row * 32 + tile) as usize];

            let tile_vram_start: usize = (2*8* (pos as usize) + (pixel_row as usize) *2) as usize;

            let byte_1 = self.vram[tile_vram_start];
            let byte_2 = self.vram[tile_vram_start+1];

            for pixel in 0..8u8 {
                let ix = 7 - pixel;
                let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

                let color: u8 = (high_bit << 1) + low_bit;
                let index: usize = (self.line as usize * 160) + (tile as usize) *8 + pixel as usize;

                self.buffer[index] = color;
            }
        }
    }

    pub fn render_buffer_to_screen(&mut self) {
    }

    pub fn step(&mut self, t: u8) {
        self.modeclock += t as u16;

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
                        self.render_buffer_to_screen();
                    }
                    else {
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
                        self.mode = 2;
                        self.line = 0;
                    }
                }
            }
            _ => { panic!("Sorry what?") }
        }

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

        assert_eq!(gpu.control, 0);

        gpu.write_byte(0xFF40, 1);

        assert_eq!(gpu.control, 1);
        assert_eq!(gpu.read_byte(0xFF40), 1);
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
}