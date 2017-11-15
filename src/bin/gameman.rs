#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

extern crate gameman;
extern crate sdl2;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

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
        rom[(0x104 + i) as usize] = byte;
    }
    memory.set_rom(rom);

    let mut cpu = CPU::new(memory);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut timer_subsystem = sdl_context.timer().unwrap();

    let window = video_subsystem.window("gameman", 600, 512)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
//    canvas.set_scale(2f32, 2f32);
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24, 256, 256).unwrap();

    let mut texture2 = texture_creator.create_texture_streaming(
        PixelFormatEnum::RGB24, 160, 144).unwrap();

    let num_clocks_in_a_frame = 70224;
    let fps = 60;
    let mut last_ticks = timer_subsystem.ticks();
    let mut pause = false;

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Q), .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => { pause ^= true; },
                _ => {}
            }
        }

        if pause { continue }
        
        let stop_clock = cpu.clks.t + num_clocks_in_a_frame;

        // step a frame forward!
        loop {
            let (_line, t) = cpu.step();
            cpu.mmu.gpu.step(t);
            if cpu.clks.t >= stop_clock {
                break
            }
        }

        canvas.clear();

        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            let mut j = 0;
            for tile in 0..384 {
                let x_offset = (tile % 32)*8;
                let y_offset = (tile / 32)*8;

                for row_of_pixel in 0..8u8 {
                    let byte_1 = cpu.mmu.read_byte(0x8000 + j);
                    let byte_2 = cpu.mmu.read_byte(0x8000 + j+1);
                    j += 2;

                    for pixel in 0..8u8 {
                        let ix = 7 - pixel;
                        let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                        let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

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

        canvas.copy(&texture, None, Some(Rect::new(0, 0, 160, 144))).unwrap();

        for tile in 0..1024u16 {
            let x_out: i32 = ((tile % 32) * 8) as i32;
            let y_out = ((tile / 32) * 8) as i32;

            let pos = cpu.mmu.read_byte(0x9800 + tile);

            let x_in = ((pos % 32) * 8) as i32;
            let y_in = ((pos / 32) * 8) as i32;

            canvas.copy(
                &texture,
                Some(Rect::new(x_in, y_in, 8, 8)),
                Some(Rect::new(x_out, 100+y_out, 8, 8))
            ).unwrap();
        }

        // draw screen!
        canvas.set_draw_color(Color::RGB(255, 0, 0));
        let scroll_y = cpu.mmu.read_byte(0xFF42);
        canvas.draw_rect(Rect::new(cpu.mmu.read_byte(0xFF43) as i32, 100+scroll_y as i32, 160, 144));
        canvas.set_draw_color(Color::RGB(0, 0, 255));
        canvas.draw_rect(Rect::new(0, 100+cpu.mmu.read_byte(0xFF44) as i32 +scroll_y as i32, 160, 1));
        canvas.set_draw_color(Color::RGB(0, 0, 0));

        texture2.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            let gpu_buffer = cpu.mmu.gpu.get_buffer();

            for y in 0..144 {
                for x in 0..160 {
                    let pixel = gpu_buffer[x + y*160];
                    let paletted_color = match pixel {
                        0x00 => { 255 }
                        0x01 => { 192 }
                        0x10 => { 96 }
                        0x11 => { 0 }
                        _ => { 128 }
                    };

                    let x_out = x * 3;
                    let y_out = y * pitch;

                    buffer[x_out + y_out] = paletted_color;
                    buffer[x_out + y_out + 1] = paletted_color;
                    buffer[x_out + y_out + 2] = paletted_color;
                }
            }
        }).unwrap();
        canvas.copy(&texture2, None, Some(Rect::new(260, 100, 160, 144))).unwrap();

        canvas.present();

        let ticks = timer_subsystem.ticks();
        let adjusted_ticks = ticks - last_ticks;
        if adjusted_ticks < 1000 / fps {
            timer_subsystem.delay((1000 / fps) - adjusted_ticks);
        }
        last_ticks = ticks;
    }

}
