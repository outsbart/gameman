use crate::gpu::GPUMemoriesAccess;
use crate::link::Link;
use crate::keypad::Key;
use crate::timers::Timers;
use cartridge::Cartridge;

pub struct MMU<M: GPUMemoriesAccess> {
    still_bios: bool,
    bios: [u8; 0x0100],

    wram: [u8; 0x2000],
    zram: [u8; 0x0080],

    pub cartridge: Box<Cartridge>,
    pub timers: Timers,

    pub interrupt_enable: u8,
    pub interrupt_flags: u8,

    pub gpu: M,
    pub key: Key,
    pub link: Link,
}

impl<M: GPUMemoriesAccess> MMU<M> {
    pub fn new(gpu: M, cartridge: Box<Cartridge>) -> MMU<M> {
        MMU {
            still_bios: false,
            bios: [0; 0x0100],

            wram: [0; 0x2000],
            zram: [0; 0x0080],

            cartridge,

            timers: Timers::new(),

            interrupt_enable: 0,
            interrupt_flags: 0xe0,

            gpu,
            key: Key::new(),
            link: Link::new(),
        }
    }

    pub fn set_bios(&mut self, bios: [u8; 0x0100]) {
        self.bios = bios;
        self.still_bios = true; // TODO: move this into a reset fn
    }

    pub fn tick_timers(&mut self, cycles: u8) {
        self.timers.tick(cycles);
    }
}

pub trait Memory {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);

    fn read_word(&mut self, addr: u16) -> u16 {
        (self.read_byte(addr) as u16) | ((self.read_byte(addr + 1) as u16) << 8)
    }

    fn write_word(&mut self, addr: u16, word: u16) -> () {
        self.write_byte(addr, (word & 0x00FF) as u8);
        self.write_byte(addr + 1, ((word & 0xFF00) >> 8) as u8);
    }
    fn tick(&mut self, _cpu_cycles: u8) {}
}

impl<M: GPUMemoriesAccess> Memory for MMU<M> {
    fn read_byte(&mut self, addr: u16) -> u8 {
        // TODO: once everything works and is tested, refactor using actual ranges
        match addr & 0xF000 {
            // BIOS
            0x0000 => {
                if self.still_bios {
                    if addr < 0x0100 {
                        return self.bios[addr as usize];
                    } else if addr == 0x0100 {
                        self.still_bios = false;
                    }
                }
                self.cartridge.read_rom(addr)
            }

            0x1000 | 0x2000 | 0x3000 => self.cartridge.read_rom(addr), // ROM 0
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.cartridge.read_rom(addr),
            0x8000 | 0x9000 => self.gpu.read_vram(addr & 0x1FFF), // VRAM
            0xA000 | 0xB000 => self.cartridge.read_ram(addr & 0x1FFF), // External RAM
            0xC000 | 0xD000 | 0xE000 => self.wram[(addr & 0x1FFF) as usize], // Working RAM

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000 | 0x0100 | 0x0200 | 0x0300 | 0x0400 |
                    0x0500 | 0x0600 | 0x0700 | 0x0800 | 0x0900 |
                    0x0A00 | 0x0B00 | 0x0C00 | 0x0D00 => self.wram[(addr & 0x1FFF) as usize], // Working RAM echo

                    // GPU OAM
                    0x0E00 => {
                        if addr & 0xFF < 0xA0  {
                            self.gpu.read_oam(addr & 0xFF)
                        } else {
                            // 0xFEA0 <= addr <= 0xFEFF, unused memory area
                            0xFF
                        }
                    }

                    // Zero page
                    0x0F00 => {
                        if addr == 0xFFFF {
                            self.interrupt_enable
                        } else if addr > 0xFF7F {
                            self.zram[(addr & 0x7F) as usize]
                        } else {
                            match addr & 0xF0 {
                                0x00 => match addr & 0xF {
                                    0 => { self.key.read_byte() }
                                    1 => { self.link.get_data() }
                                    2 => { self.link.get_control() }
                                    4 => { self.timers.read_divider() }
                                    5 => { self.timers.read_counter() }
                                    6 => { self.timers.read_modulo() }
                                    7 => { self.timers.read_control() }
                                    0xF => { self.interrupt_flags }
                                    _ => { 0 }
                                }
                                0x10 | 0x20 | 0x30 => { 0 }  // sound
                                0x40 | 0x50 | 0x60 | 0x70 => {
                                    self.gpu.read_byte(addr)
                                }
                                _ => panic!("Unhandled memory access")
                            }
                        }
                    }

                    _ => panic!("Unhandled memory access"),
                }
            }

            _ => panic!("Unhandled memory access"),
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        // TODO: once everything works and is tested, refactor using actual ranges
        match addr & 0xF000 {
            0x0000 | 0x1000 | 0x2000 | 0x3000 => self.cartridge.write_rom(addr, byte), // BIOS AND ROM 0
            0x4000 | 0x5000 | 0x6000 | 0x7000 => self.cartridge.write_rom(addr, byte), // ROM 1
            // VRAM
            0x8000 | 0x9000 => {
                self.gpu.write_vram(addr & 0x1FFF, byte);
                return;
            }
            // External RAM
            0xA000 | 0xB000 => {
                self.cartridge.write_ram(addr & 0x1FFF, byte);
                return;
            }
            // Working RAM
            0xC000 | 0xD000 | 0xE000 => {
                self.wram[(addr & 0x1FFF) as usize] = byte;
                return;
            }

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000 | 0x0100 | 0x0200 | 0x0300 | 0x0400 |
                    0x0500 | 0x0600 | 0x0700 | 0x0800 | 0x0900 |
                    0x0A00 | 0x0B00 | 0x0C00 | 0x0D00 => {
                        self.wram[(addr & 0x1FFF) as usize] = byte;
                        return;
                    }
                    // GPU OAM
                    0x0E00 => {
                        // Sprite Attribute Table (OAM - Object Attribute Memory) at $FE00-FE9F
                        if addr & 0x00FF < 0xA0 {
                            self.gpu.write_oam(addr & 0xFF, byte);
                            return;
                        } else {
                            // 0xFEA0 <= addr <= 0xFEFF, unused memory area
                            return;
                        }
                    }

                    // Zero page
                    0x0F00 => {
                        if addr == 0xFFFF {
                            self.interrupt_enable = byte;
                            return;
                        } else if addr == 0xFF0F {
                            self.interrupt_flags = byte;
                            return;
                        }
                        // keypad
                        else if addr == 0xFF00 {
                            self.key.write_byte(byte);
                            return;
                        }
                        else if addr == 0xFF01 {
                            self.link.set_data(byte);
                            return;
                        }
                        else if addr == 0xFF02 {
                            self.link.set_control(byte);
                            return;
                        }
                        else if addr == 0xFF04 {
                            self.timers.change_divider(byte);
                            return;
                        }
                        else if addr == 0xFF05 {
                            self.timers.change_counter(byte);
                            return;
                        }
                        else if addr == 0xFF06 {
                            self.timers.change_modulo(byte);
                            return;
                        }
                        else if addr == 0xFF07 {
                            self.timers.change_control(byte);
                            return;
                        }
                        else if addr >= 0xFF80 {
                            self.zram[(addr & 0x007F) as usize] = byte;
                            return;
                        }
                        else if addr >= 0xFF40 {
                            if addr == 0xFF46 {
                                // OAM DMA transfer
                                let start: u16 = (byte as u16) << 8;
                                for i in 0u16..160 {
                                    let to_be_copied = self.read_byte(start+i);
                                    self.gpu.write_oam(i, to_be_copied);
                                }
                            }
                            self.gpu.write_byte(addr, byte);
                            return;
                        }
                    }

                    _ => panic!("Unhandled memory write"),
                }
            }

            _ => panic!("Unhandled memory write"),
        }

        // println!("Memory write ignored addr=0x{:x} value={}", addr, byte);
    }

    fn tick(&mut self, cpu_cycles: u8) {
        let raise_interrupt = self.timers.tick(cpu_cycles);

        if raise_interrupt {
            let interrupt_flags = self.read_byte(0xFF0F);
            self.write_byte(0xFF0F, interrupt_flags | 4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cartridge::load_rom;

    struct DummyGPU {
        vram: [u8; 65536],
        oam: [u8; 65536],
        registers: [u8; 65536],
    }

    impl DummyGPU {
        fn new() -> DummyGPU {
            DummyGPU {
                vram: [0; 65536],
                oam: [0; 65536],
                registers: [0; 65536],
            }
        }
        fn with(vram: [u8; 65536], oam: [u8; 65536]) -> DummyGPU {
            DummyGPU {
                vram,
                oam,
                registers: [0; 65536],
            }
        }
    }

    impl GPUMemoriesAccess for DummyGPU {
        fn read_oam(&mut self, addr: u16) -> u8 {
            self.oam[addr as usize]
        }
        fn write_oam(&mut self, addr: u16, byte: u8) {
            self.oam[addr as usize] = byte;
        }
        fn read_vram(&mut self, addr: u16) -> u8 {
            self.vram[addr as usize]
        }
        fn write_vram(&mut self, addr: u16, byte: u8) {
            self.vram[addr as usize] = byte;
        }
        fn read_byte(&mut self, addr: u16) -> u8 {
            self.registers[addr as usize]
        }
        fn write_byte(&mut self, addr: u16, byte: u8) {
            self.registers[addr as usize] = byte;
        }
    }

    #[test]
    fn little_endian() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_word(0xC000, 0x1FF);
        assert_eq!(0x1FF, mmu.read_word(0xC000))
    }

    #[test]
    fn read_and_write_byte() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_byte(0xC000, 0x1);
        assert_eq!(0x1, mmu.read_byte(0xC000))
    }

    /// after instruction 0x0100 is reached,
    /// for addresses < 0x0100, rom should be accessed instead of bios
    #[test]
    fn bios_gets_replaced_by_rom() {
        // use mocks
    }

    /// test successful mapping for rom access
    /// from 0x0000 to 0x7FFF should access rom
    #[test]
    fn rom_access() {
        // use mocks
    }

    /// test successful mapping for eram access
    /// from 0xA000 to 0xBFFF should access eram
    #[test]
    fn eram_access() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        assert_eq!(mmu.read_byte(0xA000), 0xFF);
        // returns 0xFF because this rom doesnt need an eram
        // change when one is found
    }

    /// test successful mapping for eram write
    /// from 0xA000 to 0xBFFF should write to eram at addr &0x1FFF
    #[test]
    fn eram_write() {
        // use mocks
    }

    /// test successful mapping for wram access
    /// from 0xC000 to 0xFDFF should access wram
    #[test]
    fn wram_access() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );


        mmu.wram = [1; 0x2000];
        mmu.wram[0xD000 & 0x1FFF] = 2;

        assert_eq!(mmu.read_byte(0xBFFF), 0xFF);
        assert_eq!(mmu.read_byte(0xC000), 1);
        assert_eq!(mmu.read_byte(0xD000), 2);
        assert_eq!(mmu.read_byte(0xE000), 1);
        assert_eq!(mmu.read_byte(0xFDFF), 1);
        assert_eq!(mmu.read_byte(0xFE00), 0);
    }

    /// test successful mapping for wram write
    /// from 0xC000 to 0xFDFF should write to wram at addr &0x1FFF
    #[test]
    fn wram_write() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_byte(0xC000, 1);
        mmu.write_byte(0xD000, 1);
        mmu.write_byte(0xE000, 1);
        mmu.write_byte(0xFDFF, 1);

        assert_eq!(mmu.wram[0xC000 & 0x1FFF], 1);
        assert_eq!(mmu.wram[0xD000 & 0x1FFF], 1);
        assert_eq!(mmu.wram[0xE000 & 0x1FFF], 1);
        assert_eq!(mmu.wram[0xFDFF & 0x1FFF], 1);
    }

    /// test successful mapping for zero ram access
    /// from 0xFF80 to 0xFFFF should access zero ram
    /// careful, cause the areas overlaps with IO
    #[test]
    fn zram_access() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.zram = [1; 0x0080];
        mmu.zram[0xFF80 & 0x007F] = 2;

        assert_eq!(mmu.read_byte(0xFF7F), 0);
        assert_eq!(mmu.read_byte(0xFF80), 2);

        mmu.write_byte(0xFF80, 3);
        assert_eq!(mmu.read_byte(0xFF80), 3);
        assert_eq!(mmu.zram[0], 3);

        assert_eq!(mmu.read_byte(0xFF81), 1);
    }

    /// test successful mapping for zram write
    /// from 0xFF80 to 0xFFFF should write to zram at addr &0x007F
    #[test]
    fn zram_write() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_byte(0xFF80, 1);
        mmu.write_byte(0xFFB0, 1);

        assert_eq!(mmu.zram[0xFF80 & 0x007F], 1);
        assert_eq!(mmu.zram[0xFFB0 & 0x007F], 1);
    }

    /// test successful mapping for gpu vram access
    /// from 0x8000 to 0x9FFF should access gpu vram
    #[test]
    fn gpu_vram_access() {
        let mut mmu = MMU::new(
            DummyGPU::with([1; 65536], [0; 65536]),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        assert_eq!(mmu.read_byte(0x7FFF), 0);
        assert_eq!(mmu.read_byte(0x8000), 1);
        assert_eq!(mmu.read_byte(0x8000), 1);
        assert_eq!(mmu.read_byte(0x9000), 1);
        assert_eq!(mmu.read_byte(0x9FFF), 1);
        assert_eq!(mmu.read_byte(0xA000), 0xFF);
    }

    /// test successful mapping for gpu vram write
    /// from 0x8000 to 0x9FFF should write to gpu vram at addr &0x1FFF
    #[test]
    fn gpu_vram_write() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_byte(0x8000, 1);
        mmu.write_byte(0x9000, 1);
        mmu.write_byte(0x9FFF, 1);

        assert_eq!(mmu.gpu.vram[0x8000 & 0x1FFF], 1);
        assert_eq!(mmu.gpu.vram[0x9000 & 0x1FFF], 1);
        assert_eq!(mmu.gpu.vram[0x9FFF & 0x1FFF], 1);
    }

    /// test successful mapping for gpu oam access
    /// from 0xFE00 to 0xFE9F should access gpu oam
    #[test]
    fn gpu_oam_access() {
        let mut mmu = MMU::new(
            DummyGPU::with([0; 65536], [1; 65536]),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        assert_eq!(mmu.read_byte(0xFDFF), 0);
        assert_eq!(mmu.read_byte(0xFE00), 1);
        assert_eq!(mmu.read_byte(0xFE70), 1);
        assert_eq!(mmu.read_byte(0xFE9F), 1);
        assert_eq!(mmu.read_byte(0xFEA0), 0xFF);
    }

    /// test successful mapping for gpu oam write
    /// from 0xFE00 to 0xFE9F should write to gpu oam at addr &0x00FF
    #[test]
    fn gpu_oam_write() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        mmu.write_byte(0xFE00, 1);
        mmu.write_byte(0xFE70, 1);
        mmu.write_byte(0xFE9F, 1);

        assert_eq!(mmu.gpu.oam[0xFE00 & 0x00FF], 1);
        assert_eq!(mmu.gpu.oam[0xFE70 & 0x00FF], 1);
        assert_eq!(mmu.gpu.oam[0xFE9F & 0x00FF], 1);
    }

    /// test successful mapping for gpu register write
    /// from 0xFF40 to 0xFF7F should write to gpu registers
    #[test]
    fn gpu_registers_write() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );

        for i in 0u16..64u16 {
            mmu.write_byte(0xFF40 + i, 1);
        }

        assert_eq!(mmu.gpu.registers[0xFF3F], 0);
        assert_eq!(mmu.gpu.registers[0xFF80], 0);

        for i in 0u16..64u16 {
            assert_eq!(mmu.gpu.registers[(0xFF40 + i) as usize], 1);
        }
    }

    /// unmapped area (0xFEA0-0xFEFF) is unwritable and reads should always return 0xFF
    #[test]
    fn unmapped_areas() {
        let mut mmu = MMU::new(
            DummyGPU::new(),
            load_rom("tests/cpu_instrs/01-special.gb")
        );


        mmu.write_byte(0xFEA0, 0);
        assert_eq!(mmu.read_byte(0xFEA0), 0xFF);
        mmu.write_byte(0xFEB0, 0);
        assert_eq!(mmu.read_byte(0xFEB0), 0xFF);
        mmu.write_byte(0xFEC0, 0);
        assert_eq!(mmu.read_byte(0xFEC0), 0xFF);
        mmu.write_byte(0xFED0, 0);
        assert_eq!(mmu.read_byte(0xFED0), 0xFF);
        mmu.write_byte(0xFEFF, 0);
        assert_eq!(mmu.read_byte(0xFEFF), 0xFF);
    }
}
