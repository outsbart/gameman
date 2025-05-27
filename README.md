# Gameman [![Build Status](https://github.com/outsbart/gameman/actions/workflows/integration.yml/badge.svg)](https://github.com/outsbart/gameman/actions)

Gameman is a fully functional Game Boy (DMG) emulator written in Rust, originally started as a hobby project for
learning Rust and to have fun with the challenges of emulation.

<p align="center">
  <img alt="A Pokémon game running in Gameman" src="https://user-images.githubusercontent.com/3172529/67021247-a958b300-f0ff-11e9-8543-d883cf1fdbb4.png">
</p>

## Features

- **Save files** — written as `.sav` next to the ROM, loaded automatically on startup
- **Save states** — press F5 to save, F7 to load (slot 0, stored as `.ss0` next to the ROM)
- **Stereo audio** — all four channels with correct left/right panning
- **Real-Time Clock** — Pokémon Gold/Silver/Crystal in-game clocks work correctly and advance in real time
- **Classic Game Boy palette** — rendered in authentic green tones by default; the libretro frontend adds Grayscale, DMG
  Green, and GB Pocket palette options
- **Cartridge support** — all major MBCs covered
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

Download the pre-built `libgameman_libretro.so` and `gameman_libretro.info` from the [latest release](https://github.com/outsbart/gameman/releases/latest).

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

</details>

**2. Install it into RetroArch**

```bash
cp libgameman_libretro.so ~/.config/retroarch/cores/
cp gameman_libretro.info ~/.config/retroarch/cores/
```

**3. Load a game**

Open RetroArch → *Load Content* → select your `.gb` or `.gbc` ROM. Gameman will be offered automatically as a matching
core.

</details>

## Resources

- [Pan Docs](https://gbdev.io/pandocs/) — comprehensive Game Boy hardware reference
- [blargg test ROMs](https://github.com/L-P/blargg-test-roms) — CPU, timing, sound, and hardware behavior tests
- [mooneye test suite](https://github.com/Gekkio/mooneye-test-suite) — accuracy test ROMs with broad hardware coverage
- [Gambatte](https://github.com/sinamas/gambatte) — reference emulator used to verify hardware-accurate behavior
- [RetroArch](https://www.retroarch.com/) — frontend for the libretro core
