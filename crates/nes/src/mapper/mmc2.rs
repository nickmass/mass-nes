#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum Mmc2Variant {
    Mmc2,
    Mmc4,
}

impl Mmc2Variant {
    fn is_mmc4(&self) -> bool {
        matches!(self, Mmc2Variant::Mmc4)
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc2 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    variant: Mmc2Variant,
    prg_bank_count: usize,
    chr_bank_count: usize,
    prg: MappedMemory,
    chr: MappedMemory,
    prg_ram: bool,
    prg_bank: u8,
    chr_banks: [u8; 4],
    chr_latches: [u8; 2],
    mirroring: SimpleMirroring,
}

impl Mmc2 {
    pub fn new(cartridge: INes, variant: Mmc2Variant) -> Self {
        let prg_ram = cartridge.prg_ram_bytes > 0;

        let prg = if prg_ram {
            let mut prg = MappedMemory::new(&cartridge, 0x6000, 8, 40, MemKind::Prg);
            prg.map(0x6000, 8, 0, BankKind::Ram);
            prg
        } else {
            MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg)
        };

        let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
        let prg_bank_count = cartridge.prg_rom.len() / 0x2000;
        let chr_bank_count = cartridge.chr_rom.len() / 0x1000;

        let mut rom = Self {
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            variant,
            cartridge,
            prg,
            chr,
            prg_ram,
            prg_bank_count,
            chr_bank_count,
            prg_bank: 0,
            chr_banks: [0; 4],
            chr_latches: [0xfd; 2],
        };

        rom.sync();

        rom
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        let val = self.chr.read(&self.cartridge, addr);

        match addr {
            0x0fd8 => self.chr_latches[0] = 0xfd,
            0x0fe8 => self.chr_latches[0] = 0xfe,
            0x0fd8..=0x0fdf if self.variant.is_mmc4() => self.chr_latches[0] = 0xfd,
            0x0fe8..=0x0fef if self.variant.is_mmc4() => self.chr_latches[0] = 0xfe,
            0x1fd8..=0x1fdf => self.chr_latches[1] = 0xfd,
            0x1fe8..=0x1fef => self.chr_latches[1] = 0xfe,
            _ => return val,
        }

        self.sync();
        val
    }

    fn peek_ppu(&self, addr: u16) -> u8 {
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0xa000 => self.prg_bank = value,
            0xb000 => self.chr_banks[0] = value,
            0xc000 => self.chr_banks[1] = value,
            0xd000 => self.chr_banks[2] = value,
            0xe000 => self.chr_banks[3] = value,
            0xf000 if value & 1 == 0 => self.mirroring.vertical(),
            0xf000 if value & 1 == 1 => self.mirroring.horizontal(),
            0x6000..=0x7fff if self.prg_ram => self.prg.write(addr, value),
            _ => return,
        }

        self.sync();
    }

    fn sync(&mut self) {
        if self.variant.is_mmc4() {
            let bank_count = self.prg_bank_count / 2;
            self.prg.map(
                0x8000,
                16,
                (self.prg_bank & 0xf) as usize % bank_count,
                BankKind::Rom,
            );
            self.prg.map(0xc000, 16, bank_count - 1, BankKind::Rom);
        } else {
            self.prg.map(
                0x8000,
                8,
                (self.prg_bank & 0xf) as usize % self.prg_bank_count,
                BankKind::Rom,
            );
            self.prg
                .map(0xa000, 8, self.prg_bank_count - 3, BankKind::Rom);
            self.prg
                .map(0xc000, 8, self.prg_bank_count - 2, BankKind::Rom);
            self.prg
                .map(0xe000, 8, self.prg_bank_count - 1, BankKind::Rom);
        }

        let lo = if self.chr_latches[0] == 0xfd { 0 } else { 1 };
        let hi = if self.chr_latches[1] == 0xfd { 2 } else { 3 };

        self.chr.map(
            0x0000,
            4,
            (self.chr_banks[lo] & 0x1f) as usize % self.chr_bank_count,
            BankKind::Rom,
        );
        self.chr.map(
            0x1000,
            4,
            (self.chr_banks[hi] & 0x1f) as usize % self.chr_bank_count,
            BankKind::Rom,
        );
    }
}

impl Mapper for Mmc2 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xf000));

        if self.prg_ram {
            cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
            cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        }
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.peek_ppu(addr),
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

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
