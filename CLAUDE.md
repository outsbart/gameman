# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run the emulator (requires sdl feature, which is the default)
cargo run --release <rom file>

# Build
cargo build --release

# Build the libretro core
cargo build --release -p gameman-libretro

# Run all tests
cargo nextest run

# Run a single test
cargo nextest run <test_name>

# Run a specific test file
cargo nextest run --test test_mooneye_instr_timing_roms
```

## Architecture

The project is a Cargo workspace with two members:
- **`gameman`** — the core library + SDL3 binary (`src/bin/gameman.rs`)
- **`gameman-libretro`** — a `cdylib` libretro core wrapping the library

The `Gameboy` struct in `src/gameboy.rs` is the top-level orchestrator that wires everything together and owns the main
run loop. The SDL3 binary is gated behind the `sdl` feature (the default).

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

The CPU is generic over any type implementing the `Memory` trait (`src/mem.rs`). `MMU<M>` is the concrete implementation
of `Memory`, itself generic over `GPUMemoriesAccess` to allow tests to substitute a minimal GPU stub.

### Timing model

The emulator is machine-cycle accurate. Within each instruction, `CPU::tick_m()` fires four `MMU::tick_t()` calls
(one M-cycle = 4 T-cycles) after each individual memory access or internal pipeline stage, so all components stay
synchronized at M-cycle granularity. The outer loop in `Gameboy::step()` accumulates T-cycles until 70224 have elapsed
(one full frame).

### Cartridge types

Cartridge loading (`src/cartridge/mod.rs`) inspects the ROM header and returns a `Box<dyn CartridgeAccess>`. Supported
MBCs: none (ROM-only), MBC1, MBC2, MBC3, MBC5. Save files are written as `.sav` next to the ROM.

### Test ROMs

Tests live in `tests/` and run actual Game Boy ROM files against the emulator:

- **blargg tests** (`tests/blargg/`): poll the serial link buffer for `"Passed"` / `"Failed"` strings via
  `Gameboy::passes_test_rom()`, or read result byte from `0xA000` via `Gameboy::passes_blargg_ram_test_rom()`.
  Test files: `test_cpu_instr_roms`, `test_instr_timing_rom`, `test_sound_roms`, `test_halt_bug_rom`,
  `test_interrupt_time_rom`, `test_mem_timing_roms`, `test_oam_bug_roms`.
  ROM sources: https://github.com/L-P/blargg-test-roms
- **mooneye tests** (`tests/mooneye/`): detect the `LD B,B` opcode (0x40) as a test-done signal and check
  register B == 3 for pass via `Gameboy::passes_mooneye_test_rom()`.
  Test files: `test_boot_roms`, `test_mooneye_bits_roms`, `test_mooneye_instr_roms`,
  `test_mooneye_instr_timing_roms`, `test_mooneye_interrupts_roms`, `test_mooneye_mbc1_roms`,
  `test_mooneye_mbc2_roms`, `test_mooneye_mbc5_roms`, `test_mooneye_oam_dma_roms`, `test_mooneye_ppu_roms`,
  `test_mooneye_serial_roms`, `test_mooneye_timer_roms`.
  ROM sources: https://github.com/Gekkio/mooneye-test-suite/tree/main/acceptance

ROM files for integration tests are stored in `tests/`.

### Validation step

To validate your changes, run the tests with `cargo nextest run` with a timeout of 30
