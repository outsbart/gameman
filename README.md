# Gameman [![Build Status](https://github.com/outsbart/gameman/actions/workflows/integration.yml/badge.svg)](https://github.com/outsbart/gameman/actions)

Gameman is a fully functional Game Boy (DMG) / Game Boy Color (CGB) emulator written in Rust originally started as a
hobby project for
learning Rust and to have fun with the challenges of emulation.

<p align="center">
  <img alt="A Pokémon game running in Gameman" src="https://user-images.githubusercontent.com/3172529/67021247-a958b300-f0ff-11e9-8543-d883cf1fdbb4.png">
</p>

## Features

- **Games support** — Full support for Game Boy (DMG) and Game Boy Color (CGB) games
- **Colors**
    - for CGB games: full 15-bit color palettes support
    - for DMG games: automatic colorization + custom palettes available (Authentic green, Grayscale, DMG Pocket, DMG
      green)
- **Save files** — written as `.sav` next to the ROM, loaded automatically on startup
- **Save states** — press F5 to save, F7 to load (slot 0, stored as `.ss0` next to the ROM)
- **Stereo audio** — all four channels with correct left/right panning
- **Real-Time Clock** — Pokémon Gold/Silver/Crystal in-game clocks work correctly and advance in real time
- **Cartridge support** — all major types covered
- **RetroAchievements** *(libretro)* — earn achievements while you play via RetroArch
- **Cheats** *(libretro)* — GameShark and Game Genie codes via RetroArch's built-in cheat system

## Accuracy

The emulator is machine-cycle accurate. All blargg and mooneye test ROM suites pass:

| Suite   | Tests                                                                                                      |
|---------|------------------------------------------------------------------------------------------------------------|
| blargg  | `cpu_instrs`, `instr_timing`, `mem_timing`, `dmg_sound`, `halt_bug`, `interrupt_time`, `oam_bug`           |
| mooneye | `bits`, `instr`, `instr_timing`, `interrupts`, `mbc1`, `mbc2`, `mbc5`, `oam_dma`, `ppu`, `serial`, `timer` |

## Building / Running

The emulator can be run as a standalone app (using SDL3) or on any libretro frontend, such as RetroArch.

<details>
<summary><strong>Standalone (SDL3)</strong></summary>

Runs as a native desktop window.

**1. Install Rust** — [rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)

**2. Install SDL3**

| Platform        | Command                                                               |
|-----------------|-----------------------------------------------------------------------|
| Ubuntu / Debian | `sudo apt install libsdl3-dev`                                        |
| macOS           | `brew install sdl3`                                                   |
| Windows         | Download from [libsdl.org](https://wiki.libsdl.org/SDL3/Installation) |

**3. Build and run**

```bash
cargo run --release -- path/to/rom.gb
```

**4. Controls**

| D-Pad      | A | B | Select | Start | Save state | Load state |
|------------|---|---|--------|-------|------------|------------|
| Arrow keys | Z | X | A      | S     | F5         | F7         |

Save states are stored as `<rom>.ss0` next to the ROM file.

</details>

<details>
<summary><strong>Libretro / RetroArch</strong></summary>

Runs as a core inside [RetroArch](https://www.retroarch.com/), adding controller support, shaders, rewind,
RetroAchievements, and more.

**1. Get the core**

Download `gameman_libretro-linux-x86_64.zip` from
the [latest release](https://github.com/outsbart/gameman/releases/latest) and extract it. It contains two files:

- `gameman_libretro.so` — the core itself
- `gameman_libretro.info` — metadata RetroArch uses to recognise the core (supported extensions, savestates, etc.)

<details>
<summary>Or build from source</summary>

Install [Rust](https://www.rust-lang.org/tools/install) and a Clang library:

| Platform        | Command                                                         |
|-----------------|-----------------------------------------------------------------|
| Ubuntu / Debian | `sudo apt install libclang-dev`                                 |
| macOS           | `xcode-select --install` *(Clang is bundled with Xcode tools)*  |
| Windows         | Install [LLVM](https://releases.llvm.org/) and add it to `PATH` |

```bash
cargo build --release -p gameman-libretro
```

The output files are `target/release/libgameman_libretro.so` and `gameman-libretro/gameman_libretro.info`.

> **Note:** Cargo prefixes the library with `lib`, but RetroArch matches the `.info` file by exact basename. Drop the
> prefix when installing so the names line up: `libgameman_libretro.so` → `gameman_libretro.so`.

</details>

**2. Install it into RetroArch**

The `.so` and `.info` go into the folders RetroArch is configured to use — which vary by platform and install. **Don't
guess the paths; read them from RetroArch itself** under *Settings → Directory*:

| File                    | Goes into the folder shown under   |
|-------------------------|------------------------------------|
| `gameman_libretro.so`   | *Settings → Directory → Core*      |
| `gameman_libretro.info` | *Settings → Directory → Core Info* |

These may be two different folders or the same one, depending on your configuration (they correspond to
`libretro_directory` and `libretro_info_path` in `retroarch.cfg`). Copy each file into whatever path RetroArch lists
there, for example:

```bash
cp gameman_libretro.so   "<Core directory from Settings → Directory>"
cp gameman_libretro.info "<Core Info directory from Settings → Directory>"
```

If the metadata still isn't picked up after copying, and you have *Settings → Core → "Cache Core Info Files"* enabled,
delete the `core_info.cache` file in your **Core Info** directory to force a rescan — RetroArch won't re-read the info
files until that cache is cleared. (On desktop x86_64 this caching is **off** by default, so most users can skip this
step and won't have the file.)

**3. Load a game**

Open RetroArch → *Load Content* → select your `.gb` or `.gbc` ROM (both DMG and GBC games are supported). Gameman will
be offered automatically as a matching core.

</details>

## Boot ROMs (optional)

Gameman can run every game without a boot ROM, but if you supply one it plays the original Nintendo boot animation and —
with the GBC boot ROM — DMG games are colorized. Two files are recognized:

| File           | Size       | Used for                                                         |
|----------------|------------|------------------------------------------------------------------|
| `gbc_bios.bin` | 2304 bytes | All games — preferred; colorizes DMG titles                      |
| `gb_bios.bin`  | 256 bytes  | DMG games only, as a fallback when `gbc_bios.bin` is not present |

Filenames and sizes must match exactly; a missing or wrong-sized file is silently ignored and the game boots directly.
Expected MD5s: `dbfce9db9deaa2567f6a84fde55f9680` (`gbc_bios.bin`) and `32fbbd84168d3482956eb3c5051637f5`
(`gb_bios.bin`).

Where the files go depends on how you run Gameman:

- **Standalone (SDL3):** in a `boot/` directory relative to the current working directory (`boot/gbc_bios.bin`,
  `boot/gb_bios.bin`) — run the emulator from the folder that contains `boot/`.
- **Libretro / RetroArch:** in RetroArch's **System** directory; read the path from *Settings → Directory →
  System/BIOS* (`system_directory`). Both files are also listed under *Information → Core Information* as optional
  firmware.

## Resources

- [Pan Docs](https://gbdev.io/pandocs/) — comprehensive Game Boy hardware reference
- [blargg test ROMs](https://github.com/L-P/blargg-test-roms) — CPU, timing, sound, and hardware behavior tests
- [mooneye test suite](https://github.com/Gekkio/mooneye-test-suite) — accuracy test ROMs with broad hardware coverage
- [Gambatte](https://github.com/sinamas/gambatte) — reference emulator used to verify hardware-accurate behavior
- [RetroArch](https://www.retroarch.com/) — frontend for the libretro core
