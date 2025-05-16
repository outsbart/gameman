# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run the emulator
cargo run --release <rom file>

# Build
cargo build --release

# Run all tests
cargo nextest run

# Run a single test
cargo nextest run <test_name>

# Run a specific test file
cargo nextest run --test test_mooneye_instr_timing_roms
```

## Architecture

The emulator is structured as a library (`src/lib.rs`) with a thin binary entry point (`src/bin/gameman.rs`). The `Gameboy` struct in `src/gameboy.rs` is the top-level orchestrator that wires everything together and owns the main run loop.

### Component hierarchy

```
Gameboy
└── CPU<MMU<GPU>>
    └── MMU<GPU>          // Memory Management Unit
        ├── GPU            // Pixel Processing Unit + VRAM/OAM
        ├── Cartridge      // ROM + RAM + save file (trait: CartridgeAccess)
        ├── Sound          // APU
        ├── Timers         // DIV/TIMA/TMA/TAC
        ├── Key            // Joypad input
        └── Link           // Serial link (used by test ROMs for output)
```

The CPU is generic over any type implementing the `Memory` trait (`src/mem.rs`). `MMU<M>` is the concrete implementation of `Memory`, itself generic over `GPUMemoriesAccess` to allow tests to substitute a minimal GPU stub.

### Timing model

The emulator is currently instruction-accurate (not machine-cycle accurate). The main loop in `Gameboy::step()` runs CPU instructions until 70224 T-cycles per frame have elapsed. Each call to `cpu.step()` returns T-cycles consumed, which are then forwarded to all other components via `mmu.tick(t)`.

### Cartridge types

Cartridge loading (`src/cartridge/mod.rs`) inspects the ROM header and returns a `Box<dyn CartridgeAccess>`. Supported MBCs: none (ROM-only), MBC1, MBC3, MBC5. Save files are written as `.sav` next to the ROM.

### Test ROMs

Tests live in `tests/` and run actual Game Boy ROM files against the emulator:

- **blargg tests** (`test_cpu_instr_roms.rs`, `test_instr_timing_rom.rs`, `test_sound_roms.rs`): poll the serial link buffer for `"Passed"` / `"Failed"` strings via `Gameboy::passes_test_rom()`. ROM sources: https://github.com/L-P/blargg-test-roms
- **mooneye tests** (`test_mooneye_instr_timing_roms.rs`, `test_mooneye_timer_roms.rs`): detect the `LD B,B` opcode (0x40) as a test-done signal and check register B == 3 for pass via `Gameboy::passes_mooneye_test_rom()`. ROM sources: https://github.com/Gekkio/mooneye-test-suite/tree/main/acceptance

ROM files for integration tests are stored in `tests/`.
