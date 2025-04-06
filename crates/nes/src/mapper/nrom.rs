#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_ram: Option<MemoryBlock>,
}

impl Nrom {
    pub fn new(cartridge: INes) -> Nrom {
        let chr_ram =
            (cartridge.chr_ram_bytes > 0).then(|| MemoryBlock::new(cartridge.chr_ram_bytes / 1024));
        Nrom { chr_ram, cartridge }
    }
}

impl Mapper for Nrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom.read_mapped(0, 32 * 1024, addr),
            BusKind::Ppu => {
                if let Some(ram) = self.chr_ram.as_ref() {
                    ram.read_mapped(0, 8 * 1024, addr)
                } else {
                    self.cartridge.chr_rom.read_mapped(0, 8 * 1024, addr)
                }
            }
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => (),
            BusKind::Ppu => {
                if let Some(ram) = self.chr_ram.as_mut() {
                    ram.write_mapped(0, 8 * 1024, addr, value)
                }
            }
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }
}
