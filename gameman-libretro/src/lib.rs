use gameman::gameboy::Gameboy;
use gameman::keypad::Button;
use rust_libretro::{
    contexts::*,
    core::Core,
    input_descriptors,
    proc::CoreOptions,
    retro_core, sys::*, types::*,
};
use std::ffi::{CStr, CString};

const PALETTE_CLASSIC:   [u32; 4] = [0x00C4F0C2, 0x005AB9A8, 0x001E606E, 0x002D1B00];
const PALETTE_GRAYSCALE: [u32; 4] = [0x00FFFFFF, 0x00AAAAAA, 0x00555555, 0x00000000];
const PALETTE_DMG_GREEN: [u32; 4] = [0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F];
const PALETTE_POCKET:    [u32; 4] = [0x00C8C3A0, 0x008E8A6F, 0x005A5540, 0x001E1B10];

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

#[derive(CoreOptions)]
#[options({
    "gameman_palette",
    "Color Palette",
    "Selects the 4-color palette used to render the display.",
    {
        { "classic",   "Classic (Green)" },
        { "grayscale", "Grayscale"        },
        { "dmg_green", "DMG Green"        },
        { "pocket",    "GB Pocket"        },
    }
})]
struct GameboyCore {
    gameboy: Option<Gameboy>,
    rom_path: String,
    prev_buttons: JoypadState,
    pending_audio: Vec<i16>,
    palette: [u32; 4],
}

retro_core!(GameboyCore {
    gameboy: None,
    rom_path: String::new(),
    prev_buttons: JoypadState::empty(),
    pending_audio: Vec::new(),
    palette: PALETTE_CLASSIC,
});

impl Core for GameboyCore {
    fn on_init(&mut self, ctx: &mut InitContext) {
        const DESCRIPTORS: &[retro_input_descriptor] = &input_descriptors!(
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP,     "Up"     },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN,   "Down"   },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT,   "Left"   },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT,  "Right"  },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A,      "A"      },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B,      "B"      },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT, "Select" },
            { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START,  "Start"  },
        );
        let gctx: GenericContext = ctx.into();
        gctx.set_input_descriptors(DESCRIPTORS);
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        self.palette = match ctx.get_variable("gameman_palette").as_deref() {
            Some("grayscale") => PALETTE_GRAYSCALE,
            Some("dmg_green") => PALETTE_DMG_GREEN,
            Some("pocket")    => PALETTE_POCKET,
            _                 => PALETTE_CLASSIC,
        };
    }

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

        let gb = self.gameboy.as_mut().unwrap();
        let wram_ptr     = gb.cpu.mmu.wram.as_mut_ptr() as *mut _;
        let vram_ptr     = gb.cpu.mmu.gpu.vram.as_mut_ptr() as *mut _;
        let oam_ptr      = gb.cpu.mmu.gpu.oam.as_mut_ptr() as *mut _;
        let hram_ptr     = gb.cpu.mmu.zram.as_mut_ptr() as *mut _;
        let cart_ram_ptr = gb.cpu.mmu.cartridge.inner_cart().ram.as_ptr() as *mut _;
        let cart_ram_len = gb.cpu.mmu.cartridge.inner_cart().ram.len();

        let zero = unsafe { std::mem::zeroed::<retro_memory_descriptor>() };
        let mut descriptors = vec![
            retro_memory_descriptor { flags: RETRO_MEMDESC_SYSTEM_RAM as u64, ptr: wram_ptr, start: 0xC000, len: 0x2000, ..zero },
            retro_memory_descriptor { flags: RETRO_MEMDESC_VIDEO_RAM  as u64, ptr: vram_ptr, start: 0x8000, len: 0x2000, ..zero },
            retro_memory_descriptor { flags: 0,                               ptr: oam_ptr,  start: 0xFE00, select: 0xFF00, len: 0x00A0, ..zero },
            retro_memory_descriptor { flags: RETRO_MEMDESC_SYSTEM_RAM as u64, ptr: hram_ptr, start: 0xFF80, len: 0x0080, ..zero },
        ];
        if cart_ram_len > 0 {
            descriptors.push(retro_memory_descriptor {
                flags: RETRO_MEMDESC_SAVE_RAM as u64,
                ptr: cart_ram_ptr,
                start: 0xA000,
                len: cart_ram_len,
                ..zero
            });
        }
        let map = retro_memory_map { descriptors: descriptors.as_ptr(), num_descriptors: descriptors.len() as u32 };
        unsafe { ctx.set_memory_maps(map) };

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
        let pixels: Vec<u32> = fb.iter().map(|&c| self.palette[c as usize]).collect();
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
        match id {
            0 => match self.gameboy.as_mut() {
                Some(gb) => {
                    let ram = &mut gb.cpu.mmu.cartridge.inner_cart_mut().ram;
                    if ram.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        ram.as_mut_ptr() as *mut std::os::raw::c_void
                    }
                }
                None => std::ptr::null_mut(),
            },
            1 => match self.gameboy.as_mut() {
                Some(gb) => match gb.cpu.mmu.cartridge.rtc_base_secs_mut() {
                    Some(secs) => secs as *mut u64 as *mut std::os::raw::c_void,
                    None => std::ptr::null_mut(),
                },
                None => std::ptr::null_mut(),
            },
            _ => std::ptr::null_mut(),
        }
    }

    fn get_memory_size(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> usize {
        match id {
            0 => match self.gameboy.as_ref() {
                Some(gb) => gb.cpu.mmu.cartridge.inner_cart().ram.len(),
                None => 0,
            },
            1 => {
                let has_rtc = self.gameboy.as_mut()
                    .and_then(|gb| gb.cpu.mmu.cartridge.rtc_base_secs_mut())
                    .is_some();
                if has_rtc { 8 } else { 0 }
            }
            _ => 0,
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
