use cpu::is_bit_set;

/// Expose the memories of the GPU
pub trait GPUMemoriesAccess {
    fn read_oam(&mut self, addr: u16) -> u8;
    fn write_oam(&mut self, addr: u16, byte: u8);
    fn read_vram(&mut self, addr: u16) -> u8;
    fn write_vram(&mut self, addr: u16, byte: u8);
}

pub struct GPU {
    vram: [u8; 8192],
    oam: [u8; 256],
    buffer: [u8; 160 * 144],  // every pixel can have 4 values

    modeclock: u16,
    mode: u8,
    line: u8,

    scroll_x: u8,
    scroll_y: u8,
    palette: u8
}

impl GPUMemoriesAccess for GPU {
    fn read_oam(&mut self, addr: u16) -> u8 { self.oam[addr as usize] }
    fn write_oam(&mut self, addr: u16, byte: u8) { self.oam[addr as usize] = byte }
    fn read_vram(&mut self, addr: u16) -> u8 { self.vram[addr as usize] }
    fn write_vram(&mut self, addr: u16, byte: u8) { self.vram[addr as usize] = byte }
}

impl GPU {
    pub fn new() -> Self {
        GPU { vram: [0; 8192], oam: [0; 256], buffer: [0; 160 * 144], modeclock: 0, mode: 2, line: 0, scroll_x: 0, scroll_y: 0, palette: 0 }
    }

    pub fn get_buffer(&self) -> &[u8; 160*144] {
        return &self.buffer;
    }

    pub fn render_scan_to_buffer(&mut self) {
        println!("Line is {}", self.line);

        let tilemap_row: usize = self.line as usize / 8;
        let pixel_row = self.line % 8;
        let tilemap0_offset = 0x9800 - 0x8000;

        let scroll_x = 0u8;
        let scroll_y = 0u8;

        for tile in 0..20 {
            let pos = self.vram[tilemap0_offset + (tilemap_row * 32 + tile) as usize];

            let tile_vram_start: usize = (2*8* (pos as usize) + (pixel_row as usize) *2) as usize;

            let byte_1 = self.vram[tile_vram_start];
            let byte_2 = self.vram[tile_vram_start+1];

//            if pos != 0 {
//                println!("0x{:x}; pos {}; vram_start {}", 0x9800 + (tilemap_row * 32 + tile) as usize, pos, tile_vram_start);
//                println!("0x{:x}; 0x{:x}", byte_1, byte_2);
//            }

            for pixel in 0..8u8 {
                let ix = 7 - pixel;
                let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

                let color: u8 = (high_bit << 1) + low_bit;
                let index: usize = (self.line as usize * 160) + (tile as usize) *8 + pixel as usize;

                print!("{} <-- {} ", index, color);
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