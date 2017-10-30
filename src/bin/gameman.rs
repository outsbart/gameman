#![allow(dead_code)]
#![allow(unused_mut)]

extern crate gameman;
extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use gameman::utils::load_boot_rom;
use gameman::cpu::CPU;
use gameman::mem::MMU;
use gameman::gpu::GPUMemoriesAccess;


struct DummyGPU {
    vram: [u8; 65536],
    oam:  [u8; 65536]
}

impl DummyGPU {
    fn new() -> DummyGPU { DummyGPU { vram: [0; 65536], oam: [0; 65536] } }
    fn with(vram: [u8; 65536], oam: [u8; 65536]) -> DummyGPU { DummyGPU { vram, oam } }
}

impl GPUMemoriesAccess for DummyGPU {
    fn read_vram(&mut self, addr: u16) -> u8 {
        self.vram[addr as usize]
    }
    fn write_vram(&mut self, addr: u16, byte: u8) {
        self.vram[addr as usize] = byte;
    }
    fn read_oam(&mut self, addr: u16) -> u8 {
    self.oam[addr as usize]
}
    fn write_oam(&mut self, addr: u16, byte: u8) {
        self.oam[addr as usize] = byte;
    }
}

fn main() {
    let mut gpu = DummyGPU::new();
    let mut memory = MMU::new(gpu);
    memory.set_bios(load_boot_rom());
    let mut cpu = CPU::new(memory);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let scale = 3;

    let window = video_subsystem
        .window("rust-sdl2 demo: Window", 160 * scale, 144 * scale)
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    let mut tick = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();

    'mainloop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'mainloop,
                _ => {}
            }
        }

        {
            // Update the window title.
            let window = canvas.window_mut();

            let size = window.size();
            let title = format!("Window size({}x{}): {}", size.0, size.1, tick);
            window.set_title(&title).unwrap();

            tick += 1;

            cpu.step();
        }

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
    }

//    for _ in 0..1000000 {
//        cpu.step();
//    }
}
