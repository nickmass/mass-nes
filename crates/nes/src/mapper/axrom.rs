#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Axrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg: MappedMemory,
    prg_count: usize,
    chr_ram: MemoryBlock,
    mirroring: SimpleMirroring,
}

impl Axrom {
    pub fn new(cartridge: INes) -> Axrom {
        let mut prg = MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg);
        let prg_count = cartridge.prg_rom.len() / 32 * 1024;

        prg.map(0x8000, 32, 0, BankKind::Rom);

        Axrom {
            prg,
            prg_count,
            chr_ram: MemoryBlock::new(8),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr_ram.read(addr)
    }

    fn write_cpu(&mut self, _addr: u16, value: u8) {
        let bank = value as usize % self.prg_count;
        self.prg.map(0x8000, 32, bank, BankKind::Rom);
        if value & 0x10 == 0 {
            self.mirroring.internal_a()
        } else {
            self.mirroring.internal_b()
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        self.chr_ram.write(addr, value);
    }
}

impl Mapper for Axrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
