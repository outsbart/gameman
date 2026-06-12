#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    sound_01_registers,
    "tests/blargg/roms/dmg_sound/rom_singles/01-registers.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_02_len_ctr,
    "tests/blargg/roms/dmg_sound/rom_singles/02-len ctr.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_03_trigger,
    "tests/blargg/roms/dmg_sound/rom_singles/03-trigger.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_04_sweep,
    "tests/blargg/roms/dmg_sound/rom_singles/04-sweep.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_05_sweep_details,
    "tests/blargg/roms/dmg_sound/rom_singles/05-sweep details.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_06_overflow_on_trigger,
    "tests/blargg/roms/dmg_sound/rom_singles/06-overflow on trigger.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_07_len_sweep_period_sync,
    "tests/blargg/roms/dmg_sound/rom_singles/07-len sweep period sync.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_08_len_ctr_during_power,
    "tests/blargg/roms/dmg_sound/rom_singles/08-len ctr during power.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_09_wave_read_while_on,
    "tests/blargg/roms/dmg_sound/rom_singles/09-wave read while on.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_10_wave_trigger_while_on,
    "tests/blargg/roms/dmg_sound/rom_singles/10-wave trigger while on.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_11_regs_after_power,
    "tests/blargg/roms/dmg_sound/rom_singles/11-regs after power.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    sound_12_wave_write_while_on,
    "tests/blargg/roms/dmg_sound/rom_singles/12-wave write while on.gb",
    passes_blargg_ram_test_rom
);
test_rom!(
    #[ignore = "redundant with individual sub-tests which run faster in parallel"]
    sound_combined,
    "tests/blargg/roms/dmg_sound/rom_singles/dmg_sound.gb",
    passes_blargg_ram_test_rom
);
