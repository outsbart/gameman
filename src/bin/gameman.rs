#![allow(dead_code)]
#![allow(unused_mut)]

extern crate gameman;
extern crate sdl2;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use gameman::cpu::get_bit;

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
    let mut cpu = CPU::new(memory);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24, 256, 256).unwrap();
    // Create a red-green gradient
//    texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
//        for y in 0..256 {
//            for x in 0..256 {
//                let offset = y*pitch + x*3;
//                buffer[offset] = 255 as u8;
//                buffer[offset + 1] = 255 as u8;
//                buffer[offset + 2] = 255 as u8;
//            }
//        }
//    }).unwrap();
//
//    canvas.clear();
//    canvas.copy(&texture, None, Some(Rect::new(100, 100, 256, 256))).unwrap();
//    canvas.copy_ex(&texture, None,
//        Some(Rect::new(450, 100, 256, 256)), 30.0, None, false, false).unwrap();
//    canvas.present();

    // stop before executing 0x64
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
                        let mut i = 0;
                        let mut j = 0;
                        for tile in 0..256 {
                            for row_of_pixel in 0..8 {
                                let byte_1 = cpu.mmu.read_byte(0x8000 + j);
                                j+=1;
                                let byte_2 = cpu.mmu.read_byte(0x8000 + j);
                                j+=1;

                                if byte_1 != 0 {
                                    println!("0x{:x} 0x{:x}", byte_1, byte_2);

                                    i = 0;

                                    for pixel in 0..8 {
                                        let ix = 7 - pixel;
                                        let high_bit = if get_bit(ix, byte_2 as u16) { 1 } else { 0 };
                                        let low_bit = if get_bit(ix, byte_1 as u16) { 1 } else { 0 };

                                        let white: u8 = ((high_bit as u8) << 1) + (low_bit as u8);

                                        let color = match white {
                                            0x00 => { 255 }
                                            0x01 => { 192 }
                                            0x10 => { 96 }
                                            0x11 => { 0 }
                                            _ => { 128 }
                                        };

                                        let y = row_of_pixel * pitch;
                                        let x = i * 3;

                                        println!("{} {}", x, y);

                                        buffer[y + x] = color;
                                        buffer[y + x + 1] = color;
                                        buffer[y + x + 2] = color;

                                        i += 1;
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    canvas.clear();
                    canvas.copy(&texture, None, Some(Rect::new(0, 0, 800, 600))).unwrap();
                    canvas.present();
                }
                _ => {}
            }
        }
        // The rest of the game loop goes here...
    }

}
