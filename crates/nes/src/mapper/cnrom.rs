#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::Memory;
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Cnrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_bank: u8,
}

impl Cnrom {
    pub fn new(cartridge: INes) -> Cnrom {
        Cnrom {
            cartridge,
            chr_bank: 0,
        }
    }
}

impl Mapper for Cnrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom.read_mapped(0, 32 * 1024, addr),
            BusKind::Ppu => {
                self.cartridge
                    .chr_rom
                    .read_mapped(self.chr_bank as usize, 8 * 1024, addr)
            }
        }
    }

    fn write(&mut self, bus: BusKind, _addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.chr_bank = value,
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }
}
