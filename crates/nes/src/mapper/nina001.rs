#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nina001 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: Cartridge,
    prg: MappedMemory,
    prg_count: usize,
    chr: MappedMemory,
    chr_count: usize,
    mirroring: SimpleMirroring,
}

impl Nina001 {
    pub fn new(cartridge: Cartridge) -> Nina001 {
        let mut prg = MappedMemory::new(&cartridge, 0x6000, 8, 40, MemKind::Prg);

        prg.map(0x6000, 8, 0, BankKind::Ram);
        prg.map(0x8000, 32, 0, BankKind::Rom);

        let prg_count = cartridge.prg_rom.len() / 32 * 1024;

        let mut chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
        chr.map(0x0000, 4, 0, BankKind::Rom);
        chr.map(0x1000, 4, 0, BankKind::Rom);

        let chr_count = cartridge.chr_rom.len() / 4 * 1024;

        Self {
            prg,
            prg_count,
            chr,
            chr_count,
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x7ffd => {
                let bank = (value as usize) % self.prg_count;
                self.prg.map(0x8000, 32, bank, BankKind::Rom);
            }
            0x7ffe => {
                let bank = (value as usize) % self.chr_count;
                self.chr.map(0x0000, 4, bank, BankKind::Rom);
            }
            0x7fff => {
                let bank = (value as usize) % self.chr_count;
                self.chr.map(0x1000, 4, bank, BankKind::Rom);
            }
            _ => (),
        }
        self.prg.write(addr, value)
    }
}

impl Mapper for Nina001 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
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

    fn peek_ppu_fetch(&self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
