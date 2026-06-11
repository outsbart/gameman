#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(boot_regs_dmg_abc, "tests/mooneye/roms/boot_regs-dmgABC.gb", passes_mooneye_test_rom);
test_rom!(boot_div_dmg_abcmgb, "tests/mooneye/roms/boot_div-dmgABCmgb.gb", passes_mooneye_test_rom);
test_rom!(boot_hwio_dmg_abcmgb, "tests/mooneye/roms/boot_hwio-dmgABCmgb.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets DMG rev 0, not DMG ABC"] boot_regs_dmg0, "tests/mooneye/roms/boot_regs-dmg0.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets MGB (Pocket), not DMG ABC"] boot_regs_mgb, "tests/mooneye/roms/boot_regs-mgb.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets SGB, not DMG ABC"] boot_regs_sgb, "tests/mooneye/roms/boot_regs-sgb.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets SGB2, not DMG ABC"] boot_regs_sgb2, "tests/mooneye/roms/boot_regs-sgb2.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets DMG rev 0, not DMG ABC"] boot_div_dmg0, "tests/mooneye/roms/boot_div-dmg0.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets Super variants, not DMG ABC"] boot_div_s, "tests/mooneye/roms/boot_div-S.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets Super variants, not DMG ABC"] boot_div2_s, "tests/mooneye/roms/boot_div2-S.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets DMG rev 0, not DMG ABC"] boot_hwio_dmg0, "tests/mooneye/roms/boot_hwio-dmg0.gb", passes_mooneye_test_rom);
test_rom!(#[ignore = "targets Super variants, not DMG ABC"] boot_hwio_s, "tests/mooneye/roms/boot_hwio-S.gb", passes_mooneye_test_rom);
