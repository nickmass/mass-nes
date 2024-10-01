#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Cnrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: Cartridge,
    chr: MappedMemory,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Cnrom {
    pub fn new(cartridge: Cartridge) -> Cnrom {
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
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
        cpu.register_write(
            DeviceKind::Mapper,
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
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

    fn ppu_fetch(&mut self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
