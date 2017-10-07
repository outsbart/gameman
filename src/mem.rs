use gpu::GPUMemoriesAccess;

pub struct MMU<M: GPUMemoriesAccess> {
    still_bios: bool, bios: [u8; 0x0100],

    rom: [u8; 0x8000], wram: [u8; 0x2000],   // second half of rom is swappable (aka rom banking)
    eram: [u8; 0x2000], zram: [u8; 0x0080],

    gpu: M
}

impl<M: GPUMemoriesAccess> MMU<M> {
    pub fn new(gpu: M) -> MMU<M> {
        MMU {
            still_bios: true, bios: [0; 0x0100],

            rom: [0; 0x8000], wram: [0; 0x2000],
            eram: [0; 0x2000], zram: [0; 0x0080],

            gpu
        }
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
}


impl<M: GPUMemoriesAccess> Memory for MMU<M> {
    fn read_byte(&mut self, addr: u16) -> u8 {
        // TODO: once everything works and is tested, refactor using actual ranges
        match addr & 0xF000 {
            // BIOS
            0x0000 => {
                if self.still_bios {
                    if addr < 0x0100 {
                        return self.bios[addr as usize]
                    }
                    else if addr == 0x0100 {
                        self.still_bios = false;
                    }
                }
                return self.rom[addr as usize]
            }

            0x1000...0x3000 => { return self.rom[addr as usize] }                // ROM 0
            0x4000...0x7000 => { return self.rom[addr as usize] }                // TODO: banking
            0x8000 | 0x9000 => { return self.gpu.read_vram(addr &0x1FFF) } // VRAM
            0xA000 | 0xB000 => { return self.eram[(addr &0x1FFF) as usize] }     // External RAM
            0xC000...0xE000 => { return self.wram[(addr &0x1FFF) as usize] }     // Working RAM

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000...0x0D00 => { return self.wram[(addr & 0x1FFF) as usize] } // Working RAM echo

                    // GPU OAM
                    0x0E00 => {
                        if addr < 0xFEA0 { return self.gpu.read_oam(addr & 0x00FF) }
                        else { return 0 }
                    }

                    // Zero page
                    0x0F00 => {
                        if addr >= 0xFF80 { return self.zram[(addr & 0x007F) as usize] }
                        else { return 0 }  // TODO: change when IO implemented
                    }

                    _ => {
                        panic!("Unhandled memory access")
                    }
                }
            }

            _ => {
                panic!("Unhandled memory access")
            }

        }
    }
    fn write_byte(&mut self, addr: u16, byte: u8) {
        // TODO: once everything works and is tested, refactor using actual ranges
        match addr & 0xF000 {
            0x0000...0x3000 => { return }                // BIOS AND ROM 0
            0x4000...0x7000 => { return }                // ROM 1
            0x8000 | 0x9000 => { self.gpu.write_vram(addr &0x1FFF, byte); return } // VRAM
            0xA000 | 0xB000 => { self.eram[(addr &0x1FFF) as usize] = byte; return }     // External RAM
            0xC000...0xE000 => { self.wram[(addr &0x1FFF) as usize] = byte; return }     // Working RAM

            0xF000 => {
                match addr & 0x0F00 {
                    0x0000...0x0D00 => { self.wram[(addr & 0x1FFF) as usize] = byte; return } // Working RAM echo

                    // GPU OAM
                    0x0E00 => {
                        if addr < 0xFEA0 { self.gpu.write_oam(addr & 0x00FF, byte); return }
                    }

                    // Zero page
                    0x0F00 => {
                        if addr >= 0xFF80 { self.zram[(addr & 0x007F) as usize] = byte; return }
                        else { return }  // TODO: change when IO implemented
                    }

                    _ => {
                        panic!("Unhandled memory write")
                    }
                }
            }

            _ => {
                panic!("Unhandled memory write")
            }

        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    struct DummyGPU {
        vram: [u8; 65536],
        oam:  [u8; 65536]
    }

    impl DummyGPU {
        fn new() -> DummyGPU { DummyGPU { vram: [0; 65536], oam: [0; 65536] } }
        fn with(vram: [u8; 65536], oam: [u8; 65536]) -> DummyGPU { DummyGPU { vram, oam } }
    }

    impl GPUMemoriesAccess for DummyGPU {
        fn read_vram(&mut self, addr: u16) -> u8 {
            self.vram[addr as usize]
        }
        fn write_vram(&mut self, addr: u16, byte: u8) {
            self.vram[addr as usize] = byte;
        }
        fn read_oam(&mut self, addr: u16) -> u8 {
        self.oam[addr as usize]
    }
        fn write_oam(&mut self, addr: u16, byte: u8) {
            self.oam[addr as usize] = byte;
        }
    }

    #[test]
    fn little_endian() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_word(0xC000, 0x1FF);
        assert_eq!(0x1FF, mmu.read_word(0xC000))
    }

    #[test]
    fn read_and_write_byte() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0xC000, 0x1);
        assert_eq!(0x1, mmu.read_byte(0xC000))
    }

    /// after instruction 0x0100 is reached,
    /// for addresses < 0x0100, rom should be accessed instead of bios
    #[test]
    fn bios_gets_replaced_by_rom() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.rom[0x00FF] = 5;
        mmu.rom[0x0100] = 6;
        mmu.bios[0x00FF] = 3;

        assert_eq!(mmu.read_byte(0x00FF), 3);
        assert_eq!(mmu.read_byte(0x0100), 6);
        assert_eq!(mmu.read_byte(0x00FF), 5);
    }

    /// test succesful mapping for rom access
    /// from 0x0000 to 0x7FFF should access rom
    #[test]
    fn rom_access() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.rom = [1; 0x8000];
        mmu.rom[0x6000] = 2;

        assert_eq!(mmu.read_byte(0x0000), 0);
        assert_eq!(mmu.read_byte(0x3000), 1);
        assert_eq!(mmu.read_byte(0x6000), 2);
        assert_eq!(mmu.read_byte(0x7FFF), 1);
        assert_eq!(mmu.read_byte(0x8000), 0);
    }

    /// test succesful mapping for eram access
    /// from 0xA000 to 0xBFFF should access eram
    #[test]
    fn eram_access() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.eram = [1; 0x2000];
        mmu.eram[0xB000 & 0x1FFF] = 2;

        assert_eq!(mmu.read_byte(0x0FFF), 0);
        assert_eq!(mmu.read_byte(0xA000), 1);
        assert_eq!(mmu.read_byte(0xB000), 2);
        assert_eq!(mmu.read_byte(0xBFFF), 1);
        assert_eq!(mmu.read_byte(0xC000), 0);
    }

    /// test succesful mapping for eram write
    /// from 0xA000 to 0xBFFF should write to eram at addr &0x1FFF
    #[test]
    fn eram_write() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0xA000, 1);
        mmu.write_byte(0xB000, 1);
        mmu.write_byte(0xBFFF, 1);

        assert_eq!(mmu.eram[0xA000 &0x1FFF], 1);
        assert_eq!(mmu.eram[0xB000 &0x1FFF], 1);
        assert_eq!(mmu.eram[0xBFFF &0x1FFF], 1);
    }

    /// test succesful mapping for wram access
    /// from 0xC000 to 0xFDFF should access wram
    #[test]
    fn wram_access() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.wram = [1; 0x2000];
        mmu.wram[0xD000 & 0x1FFF] = 2;

        assert_eq!(mmu.read_byte(0xBFFF), 0);
        assert_eq!(mmu.read_byte(0xC000), 1);
        assert_eq!(mmu.read_byte(0xD000), 2);
        assert_eq!(mmu.read_byte(0xE000), 1);
        assert_eq!(mmu.read_byte(0xFDFF), 1);
        assert_eq!(mmu.read_byte(0xFE00), 0);
    }

    /// test succesful mapping for wram write
    /// from 0xC000 to 0xFDFF should write to wram at addr &0x1FFF
    #[test]
    fn wram_write() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0xC000, 1);
        mmu.write_byte(0xD000, 1);
        mmu.write_byte(0xE000, 1);
        mmu.write_byte(0xFDFF, 1);

        assert_eq!(mmu.wram[0xC000 &0x1FFF], 1);
        assert_eq!(mmu.wram[0xD000 &0x1FFF], 1);
        assert_eq!(mmu.wram[0xE000 &0x1FFF], 1);
        assert_eq!(mmu.wram[0xFDFF &0x1FFF], 1);
    }

    /// test succesful mapping for zero ram access
    /// from 0xFF80 to 0xFFFF should access zero ram
    #[test]
    fn zram_access() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.zram = [1; 0x0080];
        mmu.zram[0xFF80 & 0x007F] = 2;

        assert_eq!(mmu.read_byte(0xFF7F), 0);
        assert_eq!(mmu.read_byte(0xFF80), 2);
        assert_eq!(mmu.read_byte(0xFFFF), 1);
    }

    /// test succesful mapping for zram write
    /// from 0xFF80 to 0xFFFF should write to zram at addr &0x007F
    #[test]
    fn zram_write() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0xFF80, 1);
        mmu.write_byte(0xFFB0, 1);
        mmu.write_byte(0xFFFF, 1);

        assert_eq!(mmu.zram[0xFF80 &0x007F], 1);
        assert_eq!(mmu.zram[0xFFB0 &0x007F], 1);
        assert_eq!(mmu.zram[0xFFFF &0x007F], 1);
    }

    /// test succesful mapping for gpu vram access
    /// from 0x8000 to 0x9FFF should access gpu vram
    #[test]
    fn gpu_vram_access() {
        let mut mmu = MMU::new(DummyGPU::with([1; 65536], [0; 65536]));

        assert_eq!(mmu.read_byte(0x7FFF), 0);
        assert_eq!(mmu.read_byte(0x8000), 1);
        assert_eq!(mmu.read_byte(0x9000), 1);
        assert_eq!(mmu.read_byte(0x9FFF), 1);
        assert_eq!(mmu.read_byte(0xA000), 0);
    }

    /// test succesful mapping for gpu vram write
    /// from 0x8000 to 0x9FFF should write to gpu vram at addr &0x1FFF
    #[test]
    fn gpu_vram_write() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0x8000, 1);
        mmu.write_byte(0x9000, 1);
        mmu.write_byte(0x9FFF, 1);

        assert_eq!(mmu.gpu.vram[0x8000 & 0x1FFF], 1);
        assert_eq!(mmu.gpu.vram[0x9000 & 0x1FFF], 1);
        assert_eq!(mmu.gpu.vram[0x9FFF & 0x1FFF], 1);
    }

    /// test succesful mapping for gpu oam access
    /// from 0xFE00 to 0xFE9F should access gpu oam
    #[test]
    fn gpu_oam_access() {
        let mut mmu = MMU::new(DummyGPU::with([0; 65536], [1; 65536]));

        assert_eq!(mmu.read_byte(0xFDFF), 0);
        assert_eq!(mmu.read_byte(0xFE00), 1);
        assert_eq!(mmu.read_byte(0xFE70), 1);
        assert_eq!(mmu.read_byte(0xFE9F), 1);
        assert_eq!(mmu.read_byte(0xFEA0), 0);
    }

    /// test succesful mapping for gpu oam write
    /// from 0xFE00 to 0xFE9F should write to gpu oam at addr &0x00FF
    #[test]
    fn gpu_oam_write() {
        let mut mmu = MMU::new(DummyGPU::new());

        mmu.write_byte(0xFE00, 1);
        mmu.write_byte(0xFE70, 1);
        mmu.write_byte(0xFE9F, 1);

        assert_eq!(mmu.gpu.oam[0xFE00 & 0x00FF], 1);
        assert_eq!(mmu.gpu.oam[0xFE70 & 0x00FF], 1);
        assert_eq!(mmu.gpu.oam[0xFE9F & 0x00FF], 1);
    }

}