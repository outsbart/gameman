#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    #[ignore = "redundant: tests identical logic to mem_timing_2 sub-tests, which run faster in parallel"]
    mem_timing_combined,
    "tests/blargg/roms/mem_timing/mem_timing.gb",
    passes_test_rom
);
test_rom!(
    #[ignore = "redundant with individual sub-tests which run faster in parallel"]
    mem_timing_2_combined,
    "tests/blargg/roms/mem_timing-2/mem_timing.gb",
    passes_blargg_ram_test_rom
);
test_rom!(mem_timing_2_01_read_timing, "tests/blargg/roms/mem_timing-2/01-read_timing.gb", passes_blargg_ram_test_rom);
test_rom!(mem_timing_2_02_write_timing, "tests/blargg/roms/mem_timing-2/02-write_timing.gb", passes_blargg_ram_test_rom);
test_rom!(mem_timing_2_03_modify_timing, "tests/blargg/roms/mem_timing-2/03-modify_timing.gb", passes_blargg_ram_test_rom);
