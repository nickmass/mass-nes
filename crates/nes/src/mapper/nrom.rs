#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::{Mapper, SimpleMirroring};
use crate::memory::MemoryBlock;
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_ram: MemoryBlock,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Nrom {
    pub fn new(cartridge: INes) -> Nrom {
        Nrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }
}

impl Mapper for Nrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(
            DeviceKind::Mapper,
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(addr)
                } else {
                    self.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(addr)
                } else {
                    self.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => (),
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.write(addr, value);
                }
            }
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
