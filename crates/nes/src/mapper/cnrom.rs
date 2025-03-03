#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Cnrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr: MappedMemory,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Cnrom {
    pub fn new(cartridge: INes) -> Cnrom {
        let mut chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
        chr.map(0x0000, 8, 0, BankKind::Rom);

        Cnrom {
            chr,
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.cartridge.prg_rom[addr as usize]
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, _addr: u16, value: u8) {
        self.chr.map(0x0000, 8, value as usize, BankKind::Rom);
    }
}

impl Mapper for Cnrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(
            DeviceKind::Mapper,
            AndAndMask(0x8000, self.prg_len.min(0x8000) as u16 - 1),
        );
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
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
