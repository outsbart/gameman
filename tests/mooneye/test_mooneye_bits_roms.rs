#[path = "../helpers/mod.rs"]
#[macro_use]
mod helpers;
use helpers::GameboyTestExt;

use gameman::gameboy::Gameboy;

test_rom!(
    mem_oam,
    "tests/mooneye/roms/bits/mem_oam.gb",
    passes_mooneye_test_rom
);
test_rom!(
    reg_f,
    "tests/mooneye/roms/bits/reg_f.gb",
    passes_mooneye_test_rom
);
test_rom!(
    #[ignore = "GS: targets Game Boy Pocket/Super, not DMG"]
    unused_hwio_gs,
    "tests/mooneye/roms/bits/unused_hwio-GS.gb",
    passes_mooneye_test_rom
);
