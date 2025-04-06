#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Axrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_bank: u8,
    chr_ram: FixedMemoryBlock<8>,
    mirroring: SimpleMirroring,
}

impl Axrom {
    pub fn new(cartridge: INes) -> Axrom {
        Axrom {
            prg_bank: 0,
            chr_ram: FixedMemoryBlock::new(),
            mirroring: SimpleMirroring::new(cartridge.mirroring),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.cartridge
            .prg_rom
            .read_mapped(self.prg_bank as usize, 32 * 1024, addr)
    }

    fn write_cpu(&mut self, _addr: u16, value: u8) {
        self.prg_bank = value & 7;
        if value & 0x10 == 0 {
            self.mirroring.internal_a()
        } else {
            self.mirroring.internal_b()
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr_ram.read(addr)
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
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
