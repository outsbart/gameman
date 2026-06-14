extern crate sdl3;

use gameman::gameboy::Gameboy;
use gameman::keypad::Button;
use gameman::sound::AUDIO_BUFFER_SIZE;
use gameman::sound::AudioBuffer;
use gameman::sound::SAMPLE_RATE;

use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Gamepad};
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::{FRect, ScaleMode};
use sdl3::sys::joystick::SDL_JoystickID;

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

    // Optional boot ROM from the boot/ directory. Prefer the CGB boot ROM for every game
    // (it colorizes DMG titles); fall back to the DMG boot ROM only for DMG games.
    if std::path::Path::new("boot/gbc_bios.bin").exists() {
        gameboy.load_bios("boot/gbc_bios.bin");
    } else if !gameboy.cpu.mmu.gpu.cgb_mode && std::path::Path::new("boot/gb_bios.bin").exists() {
        gameboy.load_bios("boot/gb_bios.bin");
    }

    let sdl = sdl3::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let audio_subsystem = sdl.audio().unwrap();
    let gamepad_subsystem = sdl.gamepad().unwrap();

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

    // Open the first connected gamepad (if any) for MBC7 accelerometer input.
    let mut gamepad: Option<Gamepad> = gamepad_subsystem
        .gamepads()
        .ok()
        .and_then(|ids| ids.into_iter().next())
        .and_then(|id| gamepad_subsystem.open(id).ok());

    let mut audio_buf = AudioBuffer::new(AUDIO_BUFFER_SIZE);
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
                Event::ControllerDeviceAdded { which, .. } if gamepad.is_none() => {
                    gamepad = gamepad_subsystem.open(SDL_JoystickID(which)).ok();
                }
                Event::ControllerDeviceRemoved { which, .. }
                    if gamepad.as_ref().and_then(|g| g.id().ok()).map(|id| id.0) == Some(which) =>
                {
                    gamepad = None;
                }
                _ => {}
            }
        }

        if pause {
            continue;
        }

        // Feed MBC7 accelerometer from left analog stick.
        // Axis values are i16 [-32768, 32767]; map to MBC7's u16 range centered at
        // 0x8000 with ±0x2000 travel (matches the real hardware tilt range).
        if let Some(ref gp) = gamepad {
            // Map the analog stick's i16 range [-32768, 32767] to MBC7's u16 range
            // centered at 0x8000 with ±0x2000 travel (≈ hardware tilt range).
            const SCALE: f32 = 8192.0_f32 / 32767.0_f32;
            let ax = (0x8000_i32 + (f32::from(gp.axis(Axis::LeftX)) * SCALE) as i32)
                .clamp(0, 0xFFFF) as u16;
            let ay = (0x8000_i32 + (f32::from(gp.axis(Axis::LeftY)) * SCALE) as i32)
                .clamp(0, 0xFFFF) as u16;
            gameboy.set_accelerometer(ax, ay);
        }

        gameboy.step();

        canvas.clear();

        texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                let gpu_buffer = gameboy.get_framebuffer();

                for y in 0..144 {
                    for x in 0..160 {
                        let pixel = gpu_buffer[x + y * 160];
                        let r = ((pixel >> 16) & 0xFF) as u8;
                        let g = ((pixel >> 8) & 0xFF) as u8;
                        let b = (pixel & 0xFF) as u8;

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

        audio_buf.push(gameboy.drain_audio());
        while let Some(chunk) = audio_buf.take_chunk() {
            stream.put_data_i16(&chunk).unwrap();
        }

        next_frame += FRAME_DURATION;
        let now = time::Instant::now();
        if now < next_frame {
            thread::sleep(next_frame - now);
        }
    }
}
