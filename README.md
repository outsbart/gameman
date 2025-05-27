# gameman [![Build Status](https://github.com/outsbart/gameman/actions/workflows/integration.yml/badge.svg)](https://github.com/outsbart/gameman/actions)
gameman is a Game Boy (DMG) emulator written in Rust as a hobby project.
I'm doing it mostly for learning Rust and to have fun with the challenges of emulation.

<p align="center">
  <img alt="A pokemon game running in gameman" src="https://user-images.githubusercontent.com/3172529/67021247-a958b300-f0ff-11e9-8543-d883cf1fdbb4.png">
</p>

## Status
Major games like Tetris, Mario, Kirby, Zelda and Pokemon are fully working and playable.

Save files will be put in the same directory as the rom file, but with a .sav extension.

Save states are supported: press F5 to save and F7 to load (slot 0, stored as `.ss0` next to the ROM).

Audio works, but needs more testing on different platforms.

## Accuracy

The emulator is machine-cycle accurate. Each instruction fires M-cycle ticks after every individual memory access and internal pipeline stage, keeping all components (GPU, timers, APU) synchronized at M-cycle granularity.

blargg's cpu_instrs, instr_timing, mem_timing, dmg_sound test ROMs are passing.



## Cartridge support

Supported: ROM only, MBC1, MBC2, MBC3 (with Real-Time Clock for Pokémon Gold/Silver/Crystal), MBC5 (rumble silently ignored).

## Libretro / RetroArch

A libretro core (`gameman-libretro`) is available for use with RetroArch.

### Building and installing

```bash
cargo build --release -p gameman-libretro
cp target/release/libgameman_libretro.so ~/.config/retroarch/cores/
cp gameman-libretro/gameman_libretro.info ~/.config/retroarch/cores/
```

### Features

- **Save files** — cart RAM is saved and loaded by RetroArch automatically (`.srm`)
- **Save states** — full state serialization; RetroArch manages slots and rewind
- **Real-Time Clock** — MBC3 RTC state is preserved across sessions via a `.rtc` file managed by RetroArch
- **Color palettes** — selectable in Quick Menu → Options:
  - Classic (Green) *(default)*
  - Grayscale
  - DMG Green
  - GB Pocket
- **RetroAchievements** — memory descriptors expose WRAM, VRAM, OAM, HRAM and cart RAM; all achievement addresses resolve correctly
- **Cheats** — RetroArch cheat search and apply work via the same memory descriptor map

## TODO
- Refactor, refactor and refactor code
- Gameboy Color support?


## Dependencies
At the moment, SDL3 is required for sound, input and rendering.


## How to run
```bash
cargo run --release <rom location>
```

## Buttons
Use keyboard arrows for directions and...
<table style="text-align: center">
    <tr>
        <td>Gameboy</td><td>A</td><td>B</td><td>Select</td><td>Start</td>
    </tr>
    <tr>
        <td>Keyboard</td><td>Z</td><td>X</td><td>A</td><td>S</td>
    </tr>
</table>

Other keys:
| Key | Action |
|-----|--------|
| F5  | Save state (slot 0) |
| F7  | Load state (slot 0) |

