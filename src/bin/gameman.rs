#![allow(dead_code)]
#![allow(unused_mut)]

extern crate gameman;
use gameman::utils::load_boot_rom;
use gameman::cpu::CPU;
use gameman::mem::MMU;
use gameman::gpu::GPUMemoriesAccess;


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

fn main() {
    let mut gpu = DummyGPU::new();
    let mut memory = MMU::new(gpu);
    memory.set_bios(load_boot_rom());
    let mut cpu = CPU::new(memory);

    for _ in 0..100 {
        cpu.step();
    }
}
