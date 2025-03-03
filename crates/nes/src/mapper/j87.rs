#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct J87 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr: MappedMemory,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl J87 {
    pub fn new(cartridge: INes) -> J87 {
        let mut chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
        chr.map(0x0000, 8, 0, BankKind::Rom);

        J87 {
            chr,
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }
}

impl Mapper for J87 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(
            DeviceKind::Mapper,
            AndAndMask(0x8000, self.prg_len.min(0x8000) as u16 - 1),
        );
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => self.chr.read(&self.cartridge, addr),
        }
    }

    fn write(&mut self, bus: BusKind, _addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                let bank = (value & 2) >> 1 | (value & 1) << 1;
                self.chr.map(0x0000, 8, bank as usize, BankKind::Rom);
            }
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
