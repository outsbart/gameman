extern crate sdl3;

use gameman::gameboy::Gameboy;
use gameman::keypad::Button;
use gameman::sound::AUDIO_BUFFER_SIZE;
use gameman::sound::SAMPLE_RATE;

use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::{FRect, ScaleMode};

use std::{thread, time};

const SCREEN_SIZE_MULTIPLIER: u32 = 3;
const SCREEN_WIDTH: u32 = 160 * SCREEN_SIZE_MULTIPLIER;
const SCREEN_HEIGHT: u32 = 144 * SCREEN_SIZE_MULTIPLIER;
const FPS: u64 = 60;
const FRAME_DURATION: time::Duration = time::Duration::from_nanos(1_000_000_000 / FPS);

fn main() {
    let rom_path = std::env::args()
        .nth(1)
        .expect("no gb rom file given. Usage: cargo run <rom file>");

    let mut gameboy = Gameboy::new(rom_path.as_str());

    let sdl = sdl3::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let audio_subsystem = sdl.audio().unwrap();

    let spec = AudioSpec::new(
        Some(SAMPLE_RATE as i32),
        Some(2),
        Some(AudioFormat::s16_sys()),
    );

    let stream = audio_subsystem
        .open_playback_device(&spec)
        .unwrap()
        .open_device_stream(Some(&spec))
        .unwrap();

    stream.resume().unwrap();

    let window = video_subsystem
        .window("Gameman", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormat::RGB24, 160, 144)
        .unwrap();
    texture.set_scale_mode(ScaleMode::Nearest);

    let mut pending_audio: Vec<i16> = Vec::new();
    let mut next_frame = time::Instant::now();
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
                    gameboy.step();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    ..
                } => {
                    gameboy.press_button(Button::A);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::X),
                    ..
                } => {
                    gameboy.press_button(Button::B);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    gameboy.press_button(Button::SELECT);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    gameboy.press_button(Button::START);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    gameboy.press_button(Button::DOWN);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    gameboy.press_button(Button::UP);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    gameboy.press_button(Button::LEFT);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    gameboy.press_button(Button::RIGHT);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F5),
                    ..
                } => {
                    if let Err(e) = gameboy.save_state_to_file(0) {
                        eprintln!("Save state failed: {e}");
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F7),
                    ..
                } => {
                    if let Err(e) = gameboy.load_state_from_file(0) {
                        eprintln!("Load state failed: {e}");
                    }
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Z),
                    ..
                } => {
                    gameboy.release_button(Button::A);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::X),
                    ..
                } => {
                    gameboy.release_button(Button::B);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    gameboy.release_button(Button::SELECT);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    gameboy.release_button(Button::START);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    gameboy.release_button(Button::DOWN);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    gameboy.release_button(Button::UP);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    gameboy.release_button(Button::LEFT);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    gameboy.release_button(Button::RIGHT);
                }
                _ => {}
            }
        }

        if pause {
            continue;
        }

        gameboy.step();

        canvas.clear();

        texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                let gpu_buffer = gameboy.get_framebuffer();

                for y in 0..144 {
                    for x in 0..160 {
                        let pixel = gpu_buffer[x + y * 160];

                        let (r, g, b): (u8, u8, u8) = match pixel {
                            0b00 => (0xc4, 0xf0, 0xc2),
                            0b01 => (0x5a, 0xb9, 0xa8),
                            0b10 => (0x1e, 0x60, 0x6e),
                            0b11 => (0x2d, 0x1b, 0x00),
                            _ => panic!("unexpected pixel color"),
                        };

                        let x_out = x * 3;
                        let y_out = y * pitch;

                        buffer[x_out + y_out] = r;
                        buffer[x_out + y_out + 1] = g;
                        buffer[x_out + y_out + 2] = b;
                    }
                }
            })
            .unwrap();

        canvas
            .copy(
                &texture,
                None,
                FRect::new(0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32),
            )
            .unwrap();

        canvas.present();

        pending_audio.extend(gameboy.drain_audio());
        while pending_audio.len() >= AUDIO_BUFFER_SIZE {
            stream
                .put_data_i16(&pending_audio[..AUDIO_BUFFER_SIZE])
                .unwrap();
            pending_audio.drain(..AUDIO_BUFFER_SIZE);
        }

        next_frame += FRAME_DURATION;
        let now = time::Instant::now();
        if now < next_frame {
            thread::sleep(next_frame - now);
        }
    }
}
