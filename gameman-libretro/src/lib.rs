use gameman::gameboy::Gameboy;
use gameman::keypad::Button;
use rust_libretro::{
    contexts::*,
    core::{Core, CoreOptions},
    retro_core, sys::*, types::*,
};
use std::ffi::{CStr, CString};

// XRGB8888: 0x00RRGGBB — same palette as the SDL frontend
const PALETTE: [u32; 4] = [
    0x00_C4_F0_C2,
    0x00_5A_B9_A8,
    0x00_1E_60_6E,
    0x00_2D_1B_00,
];

const BUTTON_MAP: &[(JoypadState, Button)] = &[
    (JoypadState::UP, Button::UP),
    (JoypadState::DOWN, Button::DOWN),
    (JoypadState::LEFT, Button::LEFT),
    (JoypadState::RIGHT, Button::RIGHT),
    (JoypadState::A, Button::A),
    (JoypadState::B, Button::B),
    (JoypadState::SELECT, Button::SELECT),
    (JoypadState::START, Button::START),
];

struct GameboyCore {
    gameboy: Option<Gameboy>,
    rom_path: String,
    prev_buttons: JoypadState,
    pending_audio: Vec<i16>,
}

retro_core!(GameboyCore {
    gameboy: None,
    rom_path: String::new(),
    prev_buttons: JoypadState::empty(),
    pending_audio: Vec::new(),
});

impl CoreOptions for GameboyCore {}

impl Core for GameboyCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("gameman").unwrap(),
            library_version: CString::new(env!("CARGO_PKG_VERSION")).unwrap(),
            valid_extensions: CString::new("gb|gbc").unwrap(),
            need_fullpath: true,
            block_extract: false,
        }
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: 160,
                base_height: 144,
                max_width: 160,
                max_height: 144,
                aspect_ratio: 0.0,
            },
            timing: retro_system_timing {
                fps: 59.73,
                sample_rate: 44100.0,
            },
        }
    }

    fn on_load_game(
        &mut self,
        game: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ctx.set_pixel_format(PixelFormat::XRGB8888);

        let info = game.ok_or("no game info")?;
        let path = unsafe {
            if info.path.is_null() {
                return Err("no path".into());
            }
            CStr::from_ptr(info.path).to_str()?.to_owned()
        };

        self.gameboy = Some(Gameboy::new(&path));
        self.rom_path = path;
        self.prev_buttons = JoypadState::empty();
        Ok(())
    }

    fn on_unload_game(&mut self, _ctx: &mut UnloadGameContext) {
        self.gameboy = None;
    }

    fn on_reset(&mut self, _ctx: &mut ResetContext) {
        if !self.rom_path.is_empty() {
            self.gameboy = Some(Gameboy::new(&self.rom_path));
            self.prev_buttons = JoypadState::empty();
        }
    }

    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
        let gb = match self.gameboy.as_mut() {
            Some(gb) => gb,
            None => return,
        };

        // Detect button transitions
        ctx.poll_input();
        let new_buttons = ctx.get_joypad_state(0, 0);
        let pressed = new_buttons & !self.prev_buttons;
        let released = self.prev_buttons & !new_buttons;
        for &(bit, gb_btn) in BUTTON_MAP {
            if pressed.contains(bit) {
                gb.press_button(gb_btn);
            }
            if released.contains(bit) {
                gb.release_button(gb_btn);
            }
        }
        self.prev_buttons = new_buttons;

        gb.step();

        // Audio: drain mono samples, duplicate to stereo
        self.pending_audio
            .extend(gb.drain_audio().iter().flat_map(|&s| [s, s]));

        // 1478 stereo i16 values = 739 pairs ≥ 44100/59.73 ≈ 738.4 minimum
        const MIN_STEREO_SAMPLES: usize = 1478;
        let chunk: Vec<i16> = if self.pending_audio.len() >= MIN_STEREO_SAMPLES {
            self.pending_audio.drain(..MIN_STEREO_SAMPLES).collect()
        } else {
            let mut v: Vec<i16> = self.pending_audio.drain(..).collect();
            v.resize(MIN_STEREO_SAMPLES, 0);
            v
        };
        {
            let audio_ctx = AudioContext::from(&mut *ctx);
            audio_ctx.batch_audio_samples(&chunk);
        }

        // Video: palette index → XRGB8888
        let fb = gb.get_framebuffer();
        let pixels: Vec<u32> = fb.iter().map(|&c| PALETTE[c as usize]).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)
        };
        ctx.draw_frame(bytes, 160, 144, 160 * 4);
    }

    fn get_memory_data(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemoryDataContext,
    ) -> *mut std::os::raw::c_void {
        if id != 0 {
            return std::ptr::null_mut();
        }
        match self.gameboy.as_mut() {
            Some(gb) => {
                let ram = &mut gb.cpu.mmu.cartridge.inner_cart_mut().ram;
                if ram.is_empty() {
                    std::ptr::null_mut()
                } else {
                    ram.as_mut_ptr() as *mut std::os::raw::c_void
                }
            }
            None => std::ptr::null_mut(),
        }
    }

    fn get_memory_size(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> usize {
        if id != 0 {
            return 0;
        }
        match self.gameboy.as_ref() {
            Some(gb) => gb.cpu.mmu.cartridge.inner_cart().ram.len(),
            None => 0,
        }
    }

    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> usize {
        256 * 1024
    }

    fn on_serialize(&mut self, slice: &mut [u8], _ctx: &mut SerializeContext) -> bool {
        let Some(gb) = &self.gameboy else { return false; };
        match bincode::serialize(gb) {
            Ok(bytes) if bytes.len() <= slice.len() => {
                slice[..bytes.len()].copy_from_slice(&bytes);
                true
            }
            _ => false,
        }
    }

    fn on_unserialize(&mut self, slice: &mut [u8], _ctx: &mut UnserializeContext) -> bool {
        match bincode::deserialize::<Gameboy>(slice) {
            Ok(mut gb) => {
                if gb.cpu.mmu.cartridge.restore().is_ok() {
                    self.gameboy = Some(gb);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
