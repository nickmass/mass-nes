#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::memory::{BankKind, MappedMemory, MemKind};

use super::{Mapper, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nina006 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg: MappedMemory,
    chr: MappedMemory,
    mirroring: SimpleMirroring,
}

impl Nina006 {
    pub fn new(cartridge: INes) -> Self {
        let mut prg = MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg);
        let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);

        prg.map(0x8000, 32, 0, BankKind::Rom);

        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());

        Self {
            cartridge,
            prg,
            chr,
            mirroring,
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
            BusKind::Cpu => self.prg.read(&self.cartridge, addr),
            BusKind::Ppu => self.chr.read(&self.cartridge, addr),
        }
    }

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
        self.peek(bus, addr)
    }

    fn write(&mut self, bus: BusKind, _addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                let prg = (value >> 3) & 1;
                let chr = value & 7;

                self.prg.map(0x8000, 32, prg as usize, BankKind::Rom);
                self.chr.map(0x0000, 8, chr as usize, BankKind::Rom);
            }
            _ => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: crate::ppu::PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
