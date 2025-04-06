#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Bxrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_bank: u8,
    chr_ram: MemoryBlock,
}

impl Bxrom {
    pub fn new(cartridge: INes) -> Bxrom {
        Self {
            prg_bank: 0,
            chr_ram: MemoryBlock::new(8),
            cartridge,
        }
    }
}

impl Mapper for Bxrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => {
                self.cartridge
                    .prg_rom
                    .read_mapped(self.prg_bank as usize, 32 * 1024, addr)
            }
            BusKind::Ppu => self.chr_ram.read_mapped(0, 8 * 1024, addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.prg_bank = value,
            BusKind::Ppu => self.chr_ram.write_mapped(0, 8 * 1024, addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }
}
