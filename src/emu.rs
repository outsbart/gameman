#![allow(unused_must_use)]

extern crate sdl2;

use keypad::Button;

use crate::cpu::CPU;
use crate::cpu::is_bit_set;
use crate::gpu::GPU;
use crate::mem::{Memory, MMU};
use crate::cartridge::load_rom;
use crate::sound::AUDIO_BUFFER_SIZE;

use crate::utils::{load_boot_rom};
use self::sdl2::event::Event;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::Color;
use self::sdl2::pixels::PixelFormatEnum;
use self::sdl2::rect::Rect;
use self::sdl2::audio::AudioSpecDesired;
use sound::SAMPLE_RATE;
use std::{thread, time};

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const FPS: u32 = 60;
const CLOCKS_IN_A_FRAME: u32 = 70224;
const DELAY_EVERY_FRAME: u32 = 1000 / FPS;


pub struct Emulator {
    cpu: CPU<MMU<GPU>>,
}

impl Emulator {
    pub fn new(path: &str) -> Emulator {
        let cartridge = load_rom(path);
        let mmu = MMU::new(GPU::new(), cartridge);
        let cpu = CPU::new(mmu);

        Emulator {
            cpu
        }
    }

    pub fn load_bios(&mut self) {
        self.cpu.mmu.set_bios(load_boot_rom());
        self.cpu.set_registry_value("PC", 0);
    }

    fn step(&mut self) {
        let mut clocks_this_frame = 0u32;

        // step a frame forward!
        loop {
            let (_line, t) = self.cpu.step();

            clocks_this_frame += t as u32;

            let (vblank_interrupt, stat_interrupt) = self.cpu.mmu.gpu.step(t);
            if vblank_interrupt {
                self.request_vblank_interrupt();
            }
            if stat_interrupt {
                self.request_stat_interrupt();
            }
            self.cpu.mmu.sound.tick(t);

            if clocks_this_frame >= CLOCKS_IN_A_FRAME {
                break;
            }
        }
    }

    pub fn passes_test_rom(&mut self) -> bool {
        loop {
            self.step();

            let outbuffer = self.cpu.mmu.link.get_buffer();
            if outbuffer[0] != ' ' {
                let result: String = outbuffer.iter().collect();
                let passed: bool = result.contains("Passed");
                let failed: bool = result.contains("Failed");
                if passed {
                    return passed;
                }
                if failed {
                    return false;
                }
            }
        }
    }

    // TODO: move it away from here!
    fn request_keypad_interrupt(&mut self) {
        let interrupt_flags = self.cpu.mmu.read_byte(0xFF0F) | 0b10000;
        self.cpu.mmu.write_byte(0xFF0F, interrupt_flags);
    }

    // TODO: move it away from here!
    fn request_vblank_interrupt(&mut self) {
        let interrupt_flags = self.cpu.mmu.read_byte(0xFF0F) | 1;
        self.cpu.mmu.write_byte(0xFF0F, interrupt_flags);
    }

    // TODO: move it away from here!
    fn request_stat_interrupt(&mut self) {
        let interrupt_flags = self.cpu.mmu.read_byte(0xFF0F) | 2;
        self.cpu.mmu.write_byte(0xFF0F, interrupt_flags);
    }

    pub fn run(&mut self) {
        let sdl = sdl2::init().unwrap();
        let video_subsystem = sdl.video().unwrap();
        let audio_subsystem = sdl.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(1),
            samples: Some(AUDIO_BUFFER_SIZE as u16)       // default sample size
        };

        let device = audio_subsystem.open_queue::<i16, _>(None, &desired_spec).unwrap();

        let window = video_subsystem
            .window("gameman", 600, 512)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        // canvas.set_scale(2f32, 2f32);
        let texture_creator = canvas.texture_creator();

        let mut texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, 256, 256)
            .unwrap();

        let mut texture2 = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)
            .unwrap();

        let mut last_ticks = time::Instant::now();
        let mut pause = false;

        let mut event_pump = sdl.event_pump().unwrap();

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Q),
                        ..
                    }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    Event::KeyDown {
                        keycode: Some(Keycode::Space),
                        ..
                    } => {
                        pause ^= true;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::N),
                        ..
                    } => {
                        self.step();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Z),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::A);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::X),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::B);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::A),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::SELECT);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::START);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::DOWN);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::UP);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::LEFT);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyDown {
                        keycode: Some(Keycode::Right),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::RIGHT);
                        self.request_keypad_interrupt();
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::Z),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::A);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::X),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::B);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::A),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::SELECT);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::START);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::DOWN);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::UP);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::LEFT);
                    },
                    Event::KeyUp {
                        keycode: Some(Keycode::Right),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::RIGHT);
                    }
                    _ => {}
                }
            }

            if pause {
                continue;
            }

            self.step();

            canvas.clear();

            texture
                .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                    let mut j = 0;
                    for tile in 0..384 {
                        let x_offset = (tile % 32) * 8;
                        let y_offset = (tile / 32) * 8;

                        for row_of_pixel in 0..8u8 {
                            let byte_1 = self.cpu.mmu.read_byte(0x8000 + j);
                            let byte_2 = self.cpu.mmu.read_byte(0x8000 + j + 1);
                            j += 2;

                            for pixel in 0..8u8 {
                                let ix = 7 - pixel;
                                let high_bit: u8 = is_bit_set(ix, byte_2 as u16) as u8;
                                let low_bit: u8 = is_bit_set(ix, byte_1 as u16) as u8;

                                let color: u8 = (high_bit << 1) + low_bit;

                                let paletted_color = match color {
                                    0b00 => 255,
                                    0b01 => 192,
                                    0b10 => 96,
                                    0b11 => 0,
                                    _ => 128,
                                };

                                let y = (y_offset + row_of_pixel as usize) * pitch;
                                let x = (x_offset + pixel as usize) * 3;

                                buffer[y + x] = paletted_color;
                                buffer[y + x + 1] = paletted_color;
                                buffer[y + x + 2] = paletted_color;
                            }
                        }
                    }
                })
                .unwrap();

            canvas
                .copy(&texture, None, Some(Rect::new(0, 0, 320, 288)))
                .unwrap();

            for tile in 0..1024u16 {
                let x_out: i32 = ((tile % 32) * 8) as i32;
                let y_out = ((tile / 32) * 8) as i32;

                let pos = self.cpu.mmu.read_byte(0x9800 + tile);

                let x_in = ((pos % 32) * 8) as i32;
                let y_in = ((pos / 32) * 8) as i32;

                canvas
                    .copy(
                        &texture,
                        Some(Rect::new(x_in, y_in, 8, 8)),
                        Some(Rect::new(x_out, 150 + y_out, 8, 8)),
                    )
                    .unwrap();
            }

            // draw screen!
            canvas.set_draw_color(Color::RGB(255, 0, 0));
            let scroll_y = self.cpu.mmu.read_byte(0xFF42);
            let scroll_x = self.cpu.mmu.read_byte(0xFF43) as i32;
            canvas.draw_rect(Rect::new(
                scroll_x,
                150 + scroll_y as i32,
                160,
                144,
            ));
            canvas.set_draw_color(Color::RGB(0, 0, 255));
            canvas.draw_rect(Rect::new(
                scroll_x,
                150 + self.cpu.mmu.read_byte(0xFF44) as i32 + scroll_y as i32,
                160,
                1,
            ));
            canvas.set_draw_color(Color::RGB(0, 0, 0));

            texture2
                .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                    let gpu_buffer = self.cpu.mmu.gpu.get_buffer();

                    for y in 0..144 {
                        for x in 0..160 {
                            let pixel = gpu_buffer[x + y * 160];

                            let paletted_color: (u8, u8, u8) = match pixel {
                                0b00 => (0xc4, 0xf0, 0xc2),
                                0b01 => (0x5a, 0xb9, 0xa8),
                                0b10 => (0x1e, 0x60, 0x6e),
                                0b11 => (0x2d, 0x1b, 0x00),
                                _ => panic!("unexpected pixel color"),
                            };

                            let x_out = x * 3;
                            let y_out = y * pitch;

                            buffer[x_out + y_out] = paletted_color.0;
                            buffer[x_out + y_out + 1] = paletted_color.1;
                            buffer[x_out + y_out + 2] = paletted_color.2;
                        }
                    }
                })
                .unwrap();
            canvas
                .copy(&texture2, None, Some(Rect::new(260, 150, 160*2, 144*2)))
                .unwrap();

            canvas.present();

            // audio
            if self.cpu.mmu.sound.is_audio_buffer_ready() {
                let audio_buffer = self.cpu.mmu.sound.get_audio_buffer();

                // wait for device queue to drain audio buffer
                while device.size() > AUDIO_BUFFER_SIZE as u32 {
                    thread::sleep(time::Duration::from_millis(1));
                }

                device.queue(&audio_buffer[0..]);

                device.resume();
            }

            let ticks = time::Instant::now();
            let time_passed = (ticks - last_ticks).as_millis() as u32;

            if time_passed < DELAY_EVERY_FRAME {
                thread::sleep(time::Duration::from_millis((DELAY_EVERY_FRAME - time_passed) as u64));
            }

            last_ticks = ticks;
        }
    }
}
