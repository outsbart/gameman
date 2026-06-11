#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(rom_512kb, "tests/mooneye/roms/mbc5/rom_512kb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_1mb, "tests/mooneye/roms/mbc5/rom_1Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_2mb, "tests/mooneye/roms/mbc5/rom_2Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_4mb, "tests/mooneye/roms/mbc5/rom_4Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_8mb, "tests/mooneye/roms/mbc5/rom_8Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_16mb, "tests/mooneye/roms/mbc5/rom_16Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_32mb, "tests/mooneye/roms/mbc5/rom_32Mb.gb", passes_mooneye_test_rom, new_clean);
test_rom!(rom_64mb, "tests/mooneye/roms/mbc5/rom_64Mb.gb", passes_mooneye_test_rom, new_clean);
