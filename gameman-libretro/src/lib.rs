use gameman::gameboy::Gameboy;
use gameman::keypad::Button;
use rust_libretro::{
    contexts::*, core::Core, input_descriptors, proc::CoreOptions, retro_core, sys::*, types::*,
};
use std::ffi::{CStr, CString};

const PALETTE_CLASSIC: [u32; 4] = [0x00C4F0C2, 0x005AB9A8, 0x001E606E, 0x002D1B00];
const PALETTE_GRAYSCALE: [u32; 4] = [0x00FFFFFF, 0x00AAAAAA, 0x00555555, 0x00000000];
const PALETTE_DMG_GREEN: [u32; 4] = [0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F];
const PALETTE_POCKET: [u32; 4] = [0x00C8C3A0, 0x008E8A6F, 0x005A5540, 0x001E1B10];

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
    // Sensor interface for MBC7 accelerometer input (None if frontend doesn't support it).
    sensor: Option<retro_sensor_interface>,
}

/// Convert a libretro accelerometer value (m/s²) to an MBC7 axis value.
/// MBC7 is centered at 0x8000; the hardware tilt range is roughly ±1 g ≈ ±9.8 m/s²,
/// mapped here to ±0x2000 (i.e. the range 0x6000–0xA000).
fn accel_to_mbc7(g: f32) -> u16 {
    const G_MAX: f32 = 9.8;
    const HALF_RANGE: f32 = 0x2000 as f32;
    let clamped = g.clamp(-G_MAX, G_MAX);
    (0x8000_i32 + (clamped / G_MAX * HALF_RANGE) as i32).clamp(0, 0xFFFF) as u16
}

retro_core!(GameboyCore {
    gameboy: None,
    rom_path: String::new(),
    prev_buttons: JoypadState::empty(),
    pending_audio: Vec::new(),
    palette: PALETTE_CLASSIC,
    sensor: None,
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
            Some("pocket") => PALETTE_POCKET,
            _ => PALETTE_CLASSIC,
        };
        if let Some(gb) = self.gameboy.as_mut() {
            gb.set_dmg_palette(self.palette);
        }
    }

    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("Gameman").unwrap(),
            library_version: CString::new(env!("CARGO_PKG_VERSION")).unwrap(),
            valid_extensions: CString::new("gb|gbc|dmg").unwrap(),
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

        // Try to acquire the sensor interface for MBC7 accelerometer input.
        // Optional — silently ignored if the frontend doesn't support it.
        let sensor_ok = unsafe { ctx.enable_sensor_interface() }.is_ok(); // safe: only reads env callback
        if sensor_ok {
            let gctx: GenericContext = ctx.into();
            let ifaces = unsafe { gctx.interfaces() };
            if let Ok(lock) = ifaces.read() {
                if let Some(si) = lock.sensor_interface {
                    // Enable X and Y accelerometer axes at 60 Hz.
                    if let Some(set) = si.set_sensor_state {
                        unsafe {
                            set(
                                0,
                                retro_sensor_action::RETRO_SENSOR_ACCELEROMETER_ENABLE,
                                60,
                            );
                        }
                    }
                    self.sensor = Some(si);
                }
            }
        }

        let info = game.ok_or("no game info")?;
        let path = unsafe {
            if info.path.is_null() {
                return Err("no path".into());
            }
            CStr::from_ptr(info.path).to_str()?.to_owned()
        };

        let mut gb = Gameboy::new(&path);
        gb.set_dmg_palette(self.palette);

        // Optional boot ROM from RetroArch's system directory. Prefer the CGB boot ROM for every game
        // (it colorizes DMG titles); fall back to the DMG boot ROM only for DMG games when the CGB one
        // is absent. Silently skipped if missing/wrong-size (load_bios panics otherwise, which would
        // crash the frontend).
        let gctx: GenericContext = ctx.into();
        if let Some(sysdir) = unsafe {
            rust_libretro::environment::get_system_directory(*gctx.environment_callback())
        } {
            let valid = |p: &std::path::Path, len: usize| {
                std::fs::metadata(p).is_ok_and(|m| m.len() as usize == len)
            };
            let gbc = sysdir.join("gbc_bios.bin"); // 2304 bytes
            let gb_dmg = sysdir.join("gb_bios.bin"); // 256 bytes
            let chosen = if valid(&gbc, 0x0900) {
                Some(gbc)
            } else if !gb.cpu.mmu.gpu.cgb_mode && valid(&gb_dmg, 0x0100) {
                Some(gb_dmg)
            } else {
                None
            };
            if let Some(p) = chosen.as_deref().and_then(|p| p.to_str()) {
                gb.load_bios(p);
            }
        }

        self.gameboy = Some(gb);
        self.rom_path = path;
        self.prev_buttons = JoypadState::empty();

        let gb = self.gameboy.as_mut().unwrap();
        let wram_ptr = gb.cpu.mmu.wram.as_mut_ptr() as *mut _;
        let vram_ptr = gb.cpu.mmu.gpu.vram.as_mut_ptr() as *mut _;
        let oam_ptr = gb.cpu.mmu.gpu.oam.as_mut_ptr() as *mut _;
        let hram_ptr = gb.cpu.mmu.zram.as_mut_ptr() as *mut _;
        let cart_ram_ptr = gb.cpu.mmu.cartridge.inner_cart().ram.as_ptr() as *mut _;
        let cart_ram_len = gb.cpu.mmu.cartridge.inner_cart().ram.len();

        let zero = unsafe { std::mem::zeroed::<retro_memory_descriptor>() };
        let mut descriptors = vec![
            retro_memory_descriptor {
                flags: RETRO_MEMDESC_SYSTEM_RAM as u64,
                ptr: wram_ptr,
                start: 0xC000,
                len: 0x2000,
                ..zero
            },
            retro_memory_descriptor {
                flags: RETRO_MEMDESC_VIDEO_RAM as u64,
                ptr: vram_ptr,
                start: 0x8000,
                len: 0x4000,
                ..zero
            },
            retro_memory_descriptor {
                flags: 0,
                ptr: oam_ptr,
                start: 0xFE00,
                select: 0xFF00,
                len: 0x00A0,
                ..zero
            },
            retro_memory_descriptor {
                flags: RETRO_MEMDESC_SYSTEM_RAM as u64,
                ptr: hram_ptr,
                start: 0xFF80,
                len: 0x0080,
                ..zero
            },
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
        let map = retro_memory_map {
            descriptors: descriptors.as_ptr(),
            num_descriptors: descriptors.len() as u32,
        };
        unsafe { ctx.set_memory_maps(map) };

        Ok(())
    }

    fn on_unload_game(&mut self, _ctx: &mut UnloadGameContext) {
        self.gameboy = None;
    }

    fn on_cheat_reset(&mut self, _ctx: &mut CheatResetContext) {
        if let Some(gb) = self.gameboy.as_mut() {
            gb.clear_cheats();
        }
    }

    fn on_cheat_set(
        &mut self,
        _index: std::os::raw::c_uint,
        enabled: bool,
        code: &CStr,
        _ctx: &mut CheatSetContext,
    ) {
        if !enabled {
            return;
        }
        let Some(gb) = self.gameboy.as_mut() else {
            return;
        };
        // RetroArch usually sends one code per index; split defensively on common separators.
        let s = code.to_string_lossy();
        for part in s.split(['+', ';', ' ', '\n']) {
            let part = part.trim();
            if !part.is_empty() {
                // Ignore malformed codes — never panic the frontend.
                let _ = gb.add_cheat(part);
            }
        }
    }

    fn on_reset(&mut self, _ctx: &mut ResetContext) {
        if !self.rom_path.is_empty() {
            let mut gb = Gameboy::new(&self.rom_path);
            gb.set_dmg_palette(self.palette);
            self.gameboy = Some(gb);
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

        // Feed MBC7 accelerometer from the frontend's sensor interface (e.g. device gyro on
        // Android/Switch).  Values are in m/s²; ±9.8 maps to ±0x2000 around center 0x8000.
        if let Some(si) = self.sensor {
            if let Some(get) = si.get_sensor_input {
                let x = unsafe { get(0, RETRO_SENSOR_ACCELEROMETER_X) };
                let y = unsafe { get(0, RETRO_SENSOR_ACCELEROMETER_Y) };
                gb.set_accelerometer(accel_to_mbc7(x), accel_to_mbc7(y));
            }
        }

        gb.step();

        // Audio: drain interleaved stereo samples
        self.pending_audio.extend(gb.drain_audio());

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

        // Video: framebuffer is already XRGB8888
        let fb = gb.get_framebuffer();
        let bytes = unsafe { std::slice::from_raw_parts(fb.as_ptr() as *const u8, fb.len() * 4) };
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
                let has_rtc = self
                    .gameboy
                    .as_mut()
                    .and_then(|gb| gb.cpu.mmu.cartridge.rtc_base_secs_mut())
                    .is_some();
                if has_rtc { 8 } else { 0 }
            }
            _ => 0,
        }
    }

    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> usize {
        512 * 1024
    }

    fn on_serialize(&mut self, slice: &mut [u8], _ctx: &mut SerializeContext) -> bool {
        let Some(gb) = &self.gameboy else {
            return false;
        };
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
