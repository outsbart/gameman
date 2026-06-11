#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(bits_ramg, "tests/mooneye/roms/mbc2/bits_ramg.gb", passes_mooneye_test_rom, new_clean);
test_rom!(bits_romb, "tests/mooneye/roms/mbc2/bits_romb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(bits_unused, "tests/mooneye/roms/mbc2/bits_unused.gb", passes_mooneye_test_rom, new_clean);
test_rom!(ram, "tests/mooneye/roms/mbc2/ram.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_512kb, "tests/mooneye/roms/mbc2/rom_512kb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_1mb, "tests/mooneye/roms/mbc2/rom_1Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_2mb, "tests/mooneye/roms/mbc2/rom_2Mb.gb", passes_mooneye_test_rom, new_clean);
