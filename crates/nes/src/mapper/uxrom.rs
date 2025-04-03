#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, INes};
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::{Mirroring, Nametable, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Uxrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_ram: MemoryBlock,
    nt_ram: Option<[MemoryBlock; 2]>,
    mem: MappedMemory,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Uxrom {
    pub fn new(cartridge: INes) -> Uxrom {
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        let mut mem = MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg);
        mem.map(0x8000, 16, 0, BankKind::Rom);
        mem.map(0xC000, 16, last, BankKind::Rom);

        let (mirroring, nt_ram) = if cartridge.alternative_mirroring {
            match cartridge.mirroring {
                CartMirroring::Horizontal => (
                    SimpleMirroring::new(Mirroring::Single(Nametable::InternalA)),
                    None,
                ),
                CartMirroring::Vertical => (
                    SimpleMirroring::new(Mirroring::FourScreen),
                    Some([MemoryBlock::new(1), MemoryBlock::new(1)]),
                ),
            }
        } else {
            (SimpleMirroring::new(cartridge.mirroring.into()), None)
        };

        Uxrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10),
            nt_ram,
            mem,
            mirroring,
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.mem.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 != 0 {
            if let Some(nt_ram) = self.nt_ram.as_ref() {
                if addr & 0x400 == 0 {
                    nt_ram[0].read(addr & 0x3ff)
                } else {
                    nt_ram[1].read(addr & 0x3ff)
                }
            } else {
                0
            }
        } else if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.read(addr)
        } else {
            self.cartridge.chr_rom[addr as usize]
        }
    }

    fn write_cpu(&mut self, _addr: u16, value: u8) {
        self.mem.map(0x8000, 16, value as usize, BankKind::Rom);
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        if addr & 0x2000 != 0 {
            if let Some(nt_ram) = self.nt_ram.as_ref() {
                if addr & 0x400 == 0 {
                    nt_ram[0].write(addr & 0x3ff, value);
                } else {
                    nt_ram[1].write(addr & 0x3ff, value);
                }
            }
        } else if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(addr, value);
        }
    }
}

impl Mapper for Uxrom {
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
