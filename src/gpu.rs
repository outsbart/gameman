/// Expose the memories of the GPU
pub trait GPUMemoriesAccess {
    fn read_oam(&mut self, addr: u16) -> u8;
    fn write_oam(&mut self, addr: u16, byte: u8);
    fn read_vram(&mut self, addr: u16) -> u8;
    fn write_vram(&mut self, addr: u16, byte: u8);
}

pub struct GPU {
    modeclock: usize,
    mode: u8,
    line: u8
}

impl GPUMemoriesAccess for GPU {
    fn read_oam(&mut self, addr: u16) -> u8 {
        unimplemented!()
    }

    fn write_oam(&mut self, addr: u16, byte: u8) {
        unimplemented!()
    }

    fn read_vram(&mut self, addr: u16) -> u8 {
        unimplemented!()
    }

    fn write_vram(&mut self, addr: u16, byte: u8) {
        unimplemented!()
    }
}

impl GPU {
    pub fn render_scan_to_buffer(&mut self) {

    }

    pub fn render_buffer_to_screen(&mut self) {

    }

    pub fn step(&mut self, t: usize) {
        self.modeclock += t;

        match self.mode {
            2 => {
                if self.modeclock >= 80 {
                    self.modeclock = 0;
                    self.mode = 3;
                }
            }
            3 => {
                if self.modeclock >= 172 {
                    self.modeclock = 0;
                    self.mode = 0;

                    self.render_scan_to_buffer();
                }
            }
            0 => {
                if self.modeclock >= 204 {
                    self.modeclock = 0;
                    self.line += 1;

                    if self.line == 143 {
                        self.mode = 1;
                        self.render_buffer_to_screen();
                    }
                    else {
                        self.mode = 2;
                    }
                }
            }
            1 => {
                if self.modeclock >= 456 {
                    self.modeclock = 0;
                    self.line += 1;

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