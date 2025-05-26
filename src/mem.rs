use crate::cartridge::CartridgeKind;
use crate::gpu::GPUMemoriesAccess;
use crate::keypad::Key;
use crate::link::Link;
use crate::oam_dma::OamDma;
use crate::sound::Sound;
use crate::timers::Timers;

pub struct MMU<M: GPUMemoriesAccess> {
    still_bios: bool,
    bios: [u8; 0x0100],

    wram: [u8; 0x2000],
    zram: [u8; 0x0080],

    pub cartridge: CartridgeKind,
    pub timers: Timers,
    pub sound: Sound,

    interrupt_enable: u8,
    interrupt_flags: u8,

    pub oam_dma: OamDma,
    t_sub: u8, // T-cycle sub-counter within current M-cycle (0–3)
    pub gpu: M,
    pub key: Key,
    pub link: Link,
}

impl<M: GPUMemoriesAccess> MMU<M> {
    pub fn new(gpu: M, cartridge: CartridgeKind) -> MMU<M> {
        let mut mmu = MMU {
            still_bios: false,
            bios: [0; 0x0100],

            wram: [0; 0x2000],
            zram: [0; 0x0080],

            cartridge,
            sound: Sound::new(),

            timers: Timers::new(),

            interrupt_enable: 0,
            interrupt_flags: 0x01,

            oam_dma: OamDma::new(),
            t_sub: 0,
            gpu,
            key: Key::new(),
            link: Link::new(),
        };
        mmu.post_boot_init();
        mmu
    }

    fn post_boot_init(&mut self) {
        self.sound.post_boot_init();
        self.gpu.post_boot_init();
    }

    pub fn set_bios(&mut self, bios: [u8; 0x0100]) {
        self.bios = bios;
        self.still_bios = true; // TODO: move this into a reset fn
    }

    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_flags |= 1 << interrupt as u8;
    }
}

pub enum Interrupt {
    VBlank = 0,
    Stat   = 1,
    Timer  = 2,
    Serial = 3,
    Joypad = 4,
}

pub trait Memory {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, byte: u8);

    fn read_word(&mut self, addr: u16) -> u16 {
        (self.read_byte(addr) as u16) | ((self.read_byte(addr + 1) as u16) << 8)
    }

    fn write_word(&mut self, addr: u16, word: u16) {
        self.write_byte(addr, (word & 0x00FF) as u8);
        self.write_byte(addr + 1, ((word & 0xFF00) >> 8) as u8);
    }
    fn tick_t(&mut self) {}
    fn before_fetch(&mut self) {}
    fn handle_oam_corruption(&mut self, _opcode: u8, _rr: u16) {}
}

impl<M: GPUMemoriesAccess> Memory for MMU<M> {
    fn read_byte(&mut self, addr: u16) -> u8 {
        if self.oam_dma.is_active() && (0xFE00..=0xFE9F).contains(&addr) {
            return 0xFF;
        }
        match addr & 0xF000 {
            // BIOS
            0x0000 => {
                if self.still_bios && addr <= 0x00FF {
                    return self.bios[addr as usize];
                }
                if self.still_bios && addr == 0x0100 {
                    self.still_bios = false;
                }
                self.cartridge.read_rom(addr)
            }

            0x1000..=0x7000 => self.cartridge.read_rom(addr),
            0x8000 | 0x9000 => self.gpu.read_vram(addr & 0x1FFF), // VRAM
            0xA000 | 0xB000 => self.cartridge.read_ram(addr & 0x1FFF), // External RAM
            0xC000 | 0xD000 | 0xE000 => self.wram[(addr & 0x1FFF) as usize], // Working RAM

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000..=0x0D00 => self.wram[(addr & 0x1FFF) as usize], // Working RAM echo

                    // GPU OAM
                    0x0E00 => {
                        if addr & 0xFF < 0xA0 {
                            if self.gpu.oam_accessible() {
                                self.gpu.read_oam(addr & 0xFF)
                            } else {
                                0xFF
                            }
                        } else {
                            0xFF
                        }
                    }

                    // Zero page
                    0x0F00 => match addr {
                        0xFF00 => self.key.read_byte(),
                        0xFF01 => self.link.get_data(),
                        0xFF02 => self.link.get_control(),
                        0xFF04 => self.timers.read_divider(),
                        0xFF05 => self.timers.read_tima(),
                        0xFF06 => self.timers.read_tma(),
                        0xFF07 => self.timers.read_tac(),
                        0xFF0F => self.interrupt_flags | 0xE0,
                        0xFF10..=0xFF3F => self.sound.read_byte(addr),
                        0xFF46 => self.oam_dma.source,
                        0xFF40..=0xFF45 | 0xFF47..=0xFF7F => self.gpu.read_byte(addr),
                        0xFF80..=0xFFFE => self.zram[(addr & 0x7F) as usize],
                        0xFFFF => self.interrupt_enable,
                        _ => 0xFF,
                    },

                    _ => panic!("Unhandled memory access"),
                }
            }

            _ => panic!("Unhandled memory access"),
        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        match addr & 0xF000 {
            0x0000..=0x7000 => self.cartridge.write_rom(addr, byte),
            // VRAM
            0x8000 | 0x9000 => {
                self.gpu.write_vram(addr & 0x1FFF, byte);
            }
            // External RAM
            0xA000 | 0xB000 => {
                self.cartridge.write_ram(addr & 0x1FFF, byte);
            }
            // Working RAM
            0xC000 | 0xD000 | 0xE000 => {
                self.wram[(addr & 0x1FFF) as usize] = byte;
            }

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000..=0x0D00 => self.wram[(addr & 0x1FFF) as usize] = byte,
                    // GPU OAM
                    0x0E00 => {
                        // Sprite Attribute Table (OAM - Object Attribute Memory) at $FE00-FE9F
                        if addr & 0x00FF < 0xA0 {
                            let mode = self.gpu.gpu_mode();
                            let lcd_enabled = self.gpu.is_lcd_enabled();
                            if !self.oam_dma.is_active() && (!lcd_enabled || (mode != 2 && mode != 3)) {
                                self.gpu.write_oam(addr & 0xFF, byte);
                            }
                        } else {
                            // 0xFEA0 <= addr <= 0xFEFF, unused memory area
                        }
                    }

                    // Zero page
                    0x0F00 => match addr {
                        0xFFFF => self.interrupt_enable = byte,
                        0xFF0F => self.interrupt_flags = byte,
                        0xFF00 => self.key.write_byte(byte),
                        0xFF01 => self.link.set_data(byte),
                        0xFF02 => self.link.set_control(byte),
                        0xFF04 => self.timers.change_divider(byte),
                        0xFF05 => self.timers.write_tima(byte),
                        0xFF06 => self.timers.write_tma(byte),
                        0xFF07 => self.timers.write_tac(byte),
                        0xFF10..=0xFF3F => self.sound.write_byte(addr, byte),
                        0xFF46 => self.oam_dma.trigger(byte),
                        0xFF40..=0xFF45 | 0xFF47..=0xFF7F => self.gpu.write_byte(addr, byte),
                        0xFF80..=0xFFFE => self.zram[(addr & 0x007F) as usize] = byte,
                        _ => {}
                    },

                    _ => panic!("Unhandled memory write"),
                }
            }

            _ => panic!("Unhandled memory write"),
        }
    }

    fn tick_t(&mut self) {
        // DMA and sound are M-cycle gated: run their logic once every 4 T-cycles.
        self.t_sub += 1;
        if self.t_sub == 4 {
            self.t_sub = 0;
            if let Some(start) = self.oam_dma.tick_m() {
                for i in 0u16..160 {
                    let byte = self.read_byte(start + i);
                    self.gpu.write_oam(i, byte);
                }
                self.oam_dma.finish();
            }
            self.sound.tick(4);
        }
        // Timers and GPU advance every T-cycle.
        if self.timers.tick(1) {
            self.request_interrupt(Interrupt::Timer);
        }
        // Serial clock derived from bit 8 of the divider (checked after timers.tick).
        if self.link.tick(self.timers.divider()) {
            self.request_interrupt(Interrupt::Serial);
        }
        let (vblank, stat) = self.gpu.step(1);
        if vblank {
            self.request_interrupt(Interrupt::VBlank);
        }
        if stat {
            self.request_interrupt(Interrupt::Stat);
        }
    }

    fn before_fetch(&mut self) {
        self.oam_dma.snapshot(&self.gpu);
    }
    fn handle_oam_corruption(&mut self, opcode: u8, rr: u16) {
        self.oam_dma.handle_corruption(&mut self.gpu, opcode, rr);
    }
}

impl<M: GPUMemoriesAccess> MMU<M> {
    pub fn gpu_line(&self) -> u8 {
        self.gpu.get_line()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;
    use std::path::PathBuf;

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

    fn dummy_cartridge() -> CartridgeKind {
        use crate::cartridge::nombc::CartridgeNoMBC;
        CartridgeKind::NoMBC(CartridgeNoMBC::new(Cartridge::new(PathBuf::new(), vec![0; 0x8000], 0)))
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
        fn step(&mut self, _t: u8) -> (bool, bool) {
            (false, false)
        }
    }

    #[test]
    fn little_endian() {
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

        mmu.write_word(0xC000, 0x1FF);
        assert_eq!(0x1FF, mmu.read_word(0xC000))
    }

    #[test]
    fn read_and_write_byte() {
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
            dummy_cartridge(),
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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
            dummy_cartridge(),
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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

        for i in 0u16..64u16 {
            if 0xFF40 + i == 0xFF46 {
                continue;
            } // skip OAM DMA trigger
            mmu.write_byte(0xFF40 + i, 1);
        }

        assert_eq!(mmu.gpu.registers[0xFF3F], 0);
        assert_eq!(mmu.gpu.registers[0xFF40], 1);
        assert_eq!(mmu.gpu.registers[0xFF7F], 1);
        assert_eq!(mmu.gpu.registers[0xFF80], 0);

        for i in 0u16..64u16 {
            if 0xFF40 + i == 0xFF46 {
                continue;
            } // skip OAM DMA trigger
            assert_eq!(mmu.read_byte(0xFF40 + i), 1);
        }
    }

    /// unmapped area (0xFEA0-0xFEFF) is unwritable and reads should always return 0xFF
    #[test]
    fn unmapped_areas() {
        let mut mmu = MMU::new(DummyGPU::new(), dummy_cartridge());

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
