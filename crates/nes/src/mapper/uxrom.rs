#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, INes};
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

use super::{Mirroring, Nametable};

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Uxrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_banks: [u8; 2],
    chr_ram: Option<FixedMemoryBlock<8>>,
    nt_ram: Option<FixedMemoryBlock<2>>,
    mirroring: Mirroring,
}

impl Uxrom {
    pub fn new(cartridge: INes) -> Uxrom {
        let fixed_bank = ((cartridge.prg_rom.len() / 0x4000) - 1) as u8;

        let chr_ram = (cartridge.chr_ram_bytes > 0).then(|| FixedMemoryBlock::new());

        let (mirroring, nt_ram) = if cartridge.alternative_mirroring {
            match cartridge.mirroring {
                CartMirroring::Horizontal => (Mirroring::Single(Nametable::InternalA), None),
                CartMirroring::Vertical => (Mirroring::FourScreen, Some(FixedMemoryBlock::new())),
            }
        } else {
            (cartridge.mirroring.into(), None)
        };

        Uxrom {
            prg_banks: [0, fixed_bank],
            chr_ram,
            nt_ram,
            mirroring,
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        let bank_idx = match addr & 0xc000 {
            0x8000 => 0,
            0xc000 => 1,
            _ => unreachable!(),
        };
        let bank = self.prg_banks[bank_idx] as usize;
        self.cartridge.prg_rom.read_mapped(bank, 16 * 1024, addr)
    }

    fn write_cpu(&mut self, _addr: u16, value: u8) {
        self.prg_banks[0] = value;
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 != 0 {
            if let Some(nt_ram) = self.nt_ram.as_ref() {
                nt_ram.read(addr)
            } else {
                0
            }
        } else if let Some(ram) = self.chr_ram.as_ref() {
            ram.read(addr)
        } else {
            self.cartridge.chr_rom.read_mapped(0, 8 * 1024, addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr & 0x2000 != 0 {
            if let Some(nt_ram) = self.nt_ram.as_mut() {
                nt_ram.write(addr, value);
            }
        } else if let Some(ram) = self.chr_ram.as_mut() {
            ram.write(addr, value);
        }
    }
}

impl Mapper for Uxrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
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
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
