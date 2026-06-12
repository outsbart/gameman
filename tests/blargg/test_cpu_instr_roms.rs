#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    cpu_instrs_01,
    "tests/blargg/roms/cpu_instrs/01-special.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_02,
    "tests/blargg/roms/cpu_instrs/02-interrupts.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_03,
    "tests/blargg/roms/cpu_instrs/03-op sp,hl.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_04,
    "tests/blargg/roms/cpu_instrs/04-op r,imm.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_05,
    "tests/blargg/roms/cpu_instrs/05-op rp.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_06,
    "tests/blargg/roms/cpu_instrs/06-ld r,r.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_07,
    "tests/blargg/roms/cpu_instrs/07-jr,jp,call,ret,rst.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_08,
    "tests/blargg/roms/cpu_instrs/08-misc instrs.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_09,
    "tests/blargg/roms/cpu_instrs/09-op r,r.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_10,
    "tests/blargg/roms/cpu_instrs/10-bit ops.gb",
    passes_test_rom
);
test_rom!(
    cpu_instrs_11,
    "tests/blargg/roms/cpu_instrs/11-op a,(hl).gb",
    passes_test_rom
);
