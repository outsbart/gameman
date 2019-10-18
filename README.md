# gameman [![Build Status](https://travis-ci.com/outsbart/gameman.svg?branch=master)](https://travis-ci.com/outsbart/gameman)
gameman is a game boy (DMG) emulator written in rust as a hobby project.
I'm doing it mostly for learning Rust and to have fun with the challenges of emulation.

<p align="center">
  <img alt="A pokemon game running in gameman" src="https://user-images.githubusercontent.com/3172529/67021247-a958b300-f0ff-11e9-8543-d883cf1fdbb4.png">
</p>

## Status
Major games like Tetris, Kirby, Zelda and Pokemon are fully working and playable.

Audio works, but needs more testing on more platforms.

## Accuracy

Accuracy is currently at instruction level.

blargg's cpu_instrs, instrs_timing, dmg_sound test roms are passing.



## TODO
- Fix sprite rendering priority
- Machine cycle accuracy
- Properly abstract emulation code to easily allow other frontends integration
- Save states
- Refactor, refactor and refactor code
- Extend cartridge types support
- Gameboy Color support?


## Dependencies
At the moment, SDL2 is required for sound, input and rendering.


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

