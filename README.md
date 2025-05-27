# gameman [![Build Status](https://github.com/outsbart/gameman/actions/workflows/integration.yml/badge.svg)](https://github.com/outsbart/gameman/actions)
gameman is a Game Boy (DMG) emulator written in Rust as a hobby project.
I'm doing it mostly for learning Rust and to have fun with the challenges of emulation.

<p align="center">
  <img alt="A pokemon game running in gameman" src="https://user-images.githubusercontent.com/3172529/67021247-a958b300-f0ff-11e9-8543-d883cf1fdbb4.png">
</p>

## Status

Major games like Tetris, Mario, Kirby, Zelda and Pokémon are fully working and playable.

- **Save files** — written as `.sav` next to the ROM, loaded automatically on startup
- **Save states** — press F5 to save, F7 to load (slot 0, stored as `.ss0` next to the ROM)
- **Stereo audio** — all four channels with correct left/right panning
- **Real-Time Clock** — MBC3 RTC advances in real time; Pokémon Gold/Silver/Crystal clocks work correctly

## Accuracy

The emulator is machine-cycle accurate. Each instruction fires M-cycle ticks after every individual memory access and internal pipeline stage, keeping all components (GPU, timers, APU) synchronized at M-cycle granularity.

All blargg and mooneye test ROM suites pass:

| Suite | Tests |
|-------|-------|
| blargg | `cpu_instrs`, `instr_timing`, `mem_timing`, `dmg_sound`, `halt_bug`, `interrupt_time`, `oam_bug` |
| mooneye | `bits`, `instr`, `instr_timing`, `interrupts`, `mbc1`, `mbc2`, `mbc5`, `oam_dma`, `ppu`, `serial`, `timer` |

## Cartridge support

| MBC | Notes |
|-----|-------|
| ROM only | — |
| MBC1 | Multicart (MBC1M) auto-detected |
| MBC2 | Built-in 512×4-bit RAM |
| MBC3 | Real-Time Clock on cart types 0x0F / 0x10 |
| MBC5 | Rumble silently ignored |

## Frontends

gameman is structured as a core library with two frontends you can choose from:

### Standalone (SDL3)

Runs as a native window using SDL3 for rendering, audio, and input.

**Dependency:** [SDL3](https://wiki.libsdl.org/SDL3/Installation) must be installed.

```bash
cargo run --release <rom file>
```

**Controls** — keyboard arrows for directions, plus:

<table style="text-align: center">
    <tr>
        <td>Game Boy</td><td>A</td><td>B</td><td>Select</td><td>Start</td>
    </tr>
    <tr>
        <td>Keyboard</td><td>Z</td><td>X</td><td>A</td><td>S</td>
    </tr>
</table>

| Key | Action |
|-----|--------|
| F5  | Save state (slot 0) |
| F7  | Load state (slot 0) |

### Libretro / RetroArch

Runs as a core inside [RetroArch](https://www.retroarch.com/), unlocking controller support, shaders, rewind, RetroAchievements, and more.

```bash
cargo build --release -p gameman-libretro
cp target/release/libgameman_libretro.so ~/.config/retroarch/cores/
cp gameman-libretro/gameman_libretro.info ~/.config/retroarch/cores/
```

Additional features over the standalone frontend:

- **Color palettes** — choose between Classic Green, Grayscale, DMG Green, and GB Pocket in Quick Menu → Options
- **RetroAchievements** — earn achievements while you play
- **Cheats** — use RetroArch's built-in cheat system
- **RTC persistence** — the in-game clock keeps ticking between sessions

## Resources

- [Pan Docs](https://gbdev.io/pandocs/) — comprehensive Game Boy hardware reference
- [blargg test ROMs](https://github.com/L-P/blargg-test-roms) — CPU, timing, sound, and hardware behavior tests
- [mooneye test suite](https://github.com/Gekkio/mooneye-test-suite) — accuracy test ROMs with broad hardware coverage
- [Gambatte](https://github.com/sinamas/gambatte) — reference emulator used to verify hardware-accurate behavior
- [RetroArch](https://www.retroarch.com/) — frontend for the libretro core
