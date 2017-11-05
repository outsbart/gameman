#![allow(dead_code)]
#![allow(unused_mut)]

extern crate gameman;
extern crate sdl2;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use gameman::cpu::is_bit_set;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

use gameman::utils::load_boot_rom;
use gameman::cpu::CPU;
use gameman::gpu::GPU;
use gameman::mem::{MMU, Memory};


fn main() {
    let mut gpu = GPU::new();
    let mut memory = MMU::new(gpu);
    memory.set_bios(load_boot_rom());

    let mut rom: [u8; 0x8000] = [0; 0x8000];

    // copy bios logo from 0xa8 into 0x104
    for i in 0..48 {
        let byte = memory.read_byte(0xa8 + i);
//        println!("Copying 0x{:x}", byte);
        rom[(0x104 + i) as usize] = byte;
//        cpu.mmu.write_byte(0x8010 + i, byte);
    }
    memory.set_rom(rom);

    let mut cpu = CPU::new(memory);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 512, 512)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(2f32, 2f32);
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24, 256, 256).unwrap();

    // exec the bios till the part that zeros vram
    while cpu.step() != 0x1c {}

//    for i in 0..48 {
//        let byte = cpu.mmu.read_byte(0x104 + i);
//        print!("{:x} ", byte);
//    }

//    for i in 0..1000 {
//        cpu.step();
//    }

    // stop after executing 0x64
    while cpu.step() != 0x64 {}

    println!("Graphics loaded into vram!");

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                    texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                        let mut j = 0;
                        for tile in 0..384 {
                            let x_offset = (tile % 32)*8;
                            let y_offset = (tile / 32)*8;

                            for row_of_pixel in 0..8u8 {
                                let byte_1 = cpu.mmu.read_byte(0x8000 + j);
                                let byte_2 = cpu.mmu.read_byte(0x8000 + j+1);
                                j+=2;

                                for pixel in 0..8u8 {
                                    let ix = 7 - pixel;
                                    let high_bit: u8 = if is_bit_set(ix, byte_2 as u16) { 1 } else { 0 };
                                    let low_bit: u8 = if is_bit_set(ix, byte_1 as u16) { 1 } else { 0 };

                                    let color: u8 = (high_bit << 1) + low_bit;

                                    let paletted_color = match color {
                                        0x00 => { 255 }
                                        0x01 => { 192 }
                                        0x10 => { 96 }
                                        0x11 => { 0 }
                                        _ => { 128 }
                                    };

                                    let y = (y_offset + row_of_pixel as usize) * pitch;
                                    let x = (x_offset + pixel as usize) * 3;

                                    buffer[y + x] = paletted_color;
                                    buffer[y + x + 1] = paletted_color;
                                    buffer[y + x + 2] = paletted_color;
                                }
                            }
                        }
                    }).unwrap();
                    canvas.clear();
                    canvas.copy(&texture, None, Some(Rect::new(0, 0, 256, 256))).unwrap();
                    canvas.present();
                }
                _ => {}
            }
        }
        // The rest of the game loop goes here...
    }

}
