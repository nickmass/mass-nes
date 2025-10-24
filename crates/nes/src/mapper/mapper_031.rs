#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mapper031 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_ram: Option<MemoryBlock>,
    banks: [u8; 8],
}

impl Mapper031 {
    pub fn new(cartridge: INes) -> Self {
        let chr_ram =
            (cartridge.chr_ram_bytes > 0).then(|| MemoryBlock::new(cartridge.chr_ram_bytes / 1024));
        Self {
            chr_ram,
            cartridge,
            banks: [0xff; 8],
        }
    }
}

impl Mapper for Mapper031 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xf000, 0x5000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => {
                let bank_idx = (addr & 0x7fff) >> 12;
                let bank = self.banks[bank_idx as usize];
                self.cartridge
                    .prg_rom
                    .read_mapped(bank as usize, 4 * 1024, addr)
            }
            BusKind::Ppu => {
                if let Some(ram) = self.chr_ram.as_ref() {
                    ram.read_mapped(0, 8 * 1024, addr)
                } else {
                    self.cartridge.chr_rom.read_mapped(0, 8 * 1024, addr)
                }
            }
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => match addr {
                0x5000..=0x5fff => {
                    let bank = (addr & 0x7) as usize;
                    self.banks[bank] = value;
                }
                _ => (),
            },
            BusKind::Ppu => {
                if let Some(ram) = self.chr_ram.as_mut() {
                    ram.write_mapped(0, 8 * 1024, addr, value)
                }
            }
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }

    fn power(&mut self) {
        self.banks = [0xff; 8];
    }
}
