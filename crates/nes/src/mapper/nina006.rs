#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::memory::Memory;

use super::Mapper;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nina006 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_bank: u8,
    chr_bank: u8,
}

impl Nina006 {
    pub fn new(cartridge: INes) -> Self {
        Self {
            cartridge,
            prg_bank: 0,
            chr_bank: 0,
        }
    }
}

impl Mapper for Nina006 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xE100, 0x4100, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => {
                self.cartridge
                    .prg_rom
                    .read_mapped(self.prg_bank as usize, 32 * 1024, addr)
            }
            BusKind::Ppu => {
                self.cartridge
                    .chr_rom
                    .read_mapped(self.chr_bank as usize, 8 * 1024, addr)
            }
        }
    }

    fn write(&mut self, bus: BusKind, _addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                self.prg_bank = (value >> 3) & 1;
                self.chr_bank = value & 7;
            }
            _ => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: crate::ppu::PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }
}
