#![allow(unused_must_use)]

extern crate sdl2;

use keypad::Button;

use crate::cartridge::load_rom;
use crate::cpu::CPU;
use crate::gpu::GPU;
use crate::mem::{Memory, MMU};
use crate::sound::AUDIO_BUFFER_SIZE;

use self::sdl2::audio::AudioSpecDesired;
use self::sdl2::event::Event;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::PixelFormatEnum;
use self::sdl2::rect::Rect;
use crate::utils::load_boot_rom;
use sound::SAMPLE_RATE;
use std::{thread, time};

const SCREEN_SIZE_MULTIPLIER: u32 = 3;
const SCREEN_WIDTH: u32 = 160 * SCREEN_SIZE_MULTIPLIER;
const SCREEN_HEIGHT: u32 = 144 * SCREEN_SIZE_MULTIPLIER;
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

        Emulator { cpu }
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
            samples: Some(AUDIO_BUFFER_SIZE as u16), // default sample size
        };

        let device = audio_subsystem
            .open_queue::<i16, _>(None, &desired_spec)
            .unwrap();

        let window = video_subsystem
            .window("gameman", SCREEN_WIDTH, SCREEN_HEIGHT)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        // canvas.set_scale(2f32, 2f32);
        let texture_creator = canvas.texture_creator();

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
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::X),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::B);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::A),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::SELECT);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::START);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::DOWN);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::UP);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::LEFT);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Right),
                        ..
                    } => {
                        self.cpu.mmu.key.press(Button::RIGHT);
                        self.request_keypad_interrupt();
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Z),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::A);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::X),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::B);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::A),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::SELECT);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::START);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Down),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::DOWN);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Up),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::UP);
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::Left),
                        ..
                    } => {
                        self.cpu.mmu.key.release(Button::LEFT);
                    }
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
                .copy(
                    &texture2,
                    None,
                    Some(Rect::new(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT)),
                )
                .unwrap();

            canvas.present();

            // audio
            if let Some(audio_buffer) = self.cpu.mmu.sound.get_audio_buffer() {
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
                thread::sleep(time::Duration::from_millis(
                    (DELAY_EVERY_FRAME - time_passed) as u64,
                ));
            }

            last_ticks = ticks;
        }
    }
}
