#[macro_use]
extern crate libretro_backend;

use libretro_backend::{AudioVideoInfo, Core, CoreInfo, GameData, JoypadButton, LoadGameResult, PixelFormat, RuntimeHandle};
use gameman::gameboy::Gameboy;
use gameman::keypad::Button;

// XRGB8888: 0x00RRGGBB — same palette as the SDL frontend
const PALETTE: [u32; 4] = [
    0x00_C4_F0_C2,
    0x00_5A_B9_A8,
    0x00_1E_60_6E,
    0x00_2D_1B_00,
];

const BUTTON_MAP: &[(JoypadButton, Button)] = &[
    (JoypadButton::Up,     Button::UP),
    (JoypadButton::Down,   Button::DOWN),
    (JoypadButton::Left,   Button::LEFT),
    (JoypadButton::Right,  Button::RIGHT),
    (JoypadButton::A,      Button::A),
    (JoypadButton::B,      Button::B),
    (JoypadButton::Select, Button::SELECT),
    (JoypadButton::Start,  Button::START),
];

struct GameboyCore {
    gameboy: Option<Gameboy>,
    game_data: Option<GameData>,
    rom_path: String,
    prev_buttons: u8,
    pending_audio: Vec<i16>,
}

impl Default for GameboyCore {
    fn default() -> Self {
        GameboyCore {
            gameboy: None,
            game_data: None,
            rom_path: String::new(),
            prev_buttons: 0,
            pending_audio: Vec::new(),
        }
    }
}

impl Core for GameboyCore {
    fn info() -> CoreInfo {
        CoreInfo::new("gameman", env!("CARGO_PKG_VERSION"))
            .supports_roms_with_extension("gb")
            .requires_path_when_loading_roms()
    }

    fn on_load_game(&mut self, game_data: GameData) -> LoadGameResult {
        let path = match game_data.path() {
            Some(p) => p.to_string(),
            None => return LoadGameResult::Failed(game_data),
        };

        self.gameboy = Some(Gameboy::new(&path));
        self.rom_path = path;
        self.prev_buttons = 0;
        self.game_data = Some(game_data);

        LoadGameResult::Success(
            AudioVideoInfo::new()
                .video(160, 144, 59.73, PixelFormat::ARGB8888)
                .audio(44100.0),
        )
    }

    fn on_unload_game(&mut self) -> GameData {
        self.gameboy = None;
        self.game_data.take().unwrap()
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {
        let gb = match self.gameboy.as_mut() {
            Some(gb) => gb,
            None => return,
        };

        // Detect button transitions and drive the keypad
        let mut new_buttons: u8 = 0;
        for (i, (retro_btn, gb_btn)) in BUTTON_MAP.iter().enumerate() {
            let pressed = handle.is_joypad_button_pressed(0, *retro_btn);
            if pressed {
                new_buttons |= 1 << i;
            }
            let was = (self.prev_buttons >> i) & 1 != 0;
            if pressed && !was {
                gb.press_button(*gb_btn);
            } else if !pressed && was {
                gb.release_button(*gb_btn);
            }
        }
        self.prev_buttons = new_buttons;

        gb.step();

        // Drain all mono samples produced this frame, duplicate to stereo
        let mono = gb.drain_audio();
        self.pending_audio.extend(mono.iter().flat_map(|&s| [s, s]));

        // Upload >= minimum required (1478 = 739 stereo pairs > 1476.64 minimum)
        const MIN_STEREO_SAMPLES: usize = 1478;
        let chunk = if self.pending_audio.len() >= MIN_STEREO_SAMPLES {
            self.pending_audio.drain(..MIN_STEREO_SAMPLES).collect()
        } else {
            // Pad with silence — only happens for the first frame at startup
            let mut v: Vec<i16> = self.pending_audio.drain(..).collect();
            v.resize(MIN_STEREO_SAMPLES, 0);
            v
        };
        handle.upload_audio_frame(&chunk);

        // Video: palette index 0-3 → XRGB8888 bytes
        let fb = gb.get_framebuffer();
        let pixels: Vec<u32> = fb.iter().map(|&c| PALETTE[c as usize]).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)
        };
        handle.upload_video_frame(bytes);
    }

    fn on_reset(&mut self) {
        if !self.rom_path.is_empty() {
            self.gameboy = Some(Gameboy::new(&self.rom_path));
            self.prev_buttons = 0;
        }
    }
}

libretro_core!(GameboyCore);
