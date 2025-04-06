#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Bf909x {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_bank: u8,
    prg_last_bank: usize,
    chr_ram: Option<MemoryBlock>,
    mirroring: SimpleMirroring,
}

impl Bf909x {
    pub fn new(cartridge: INes) -> Bf909x {
        let prg_last_bank = (cartridge.prg_rom.len() / (16 * 1024)) - 1;

        let chr_ram = (cartridge.chr_ram_bytes >= 1024)
            .then(|| MemoryBlock::new(cartridge.chr_ram_bytes >> 10));

        Bf909x {
            prg_bank: 0,
            prg_last_bank,
            chr_ram,
            mirroring: SimpleMirroring::new(cartridge.mirroring),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        let bank = match addr & 0xc000 {
            0xc000 => self.prg_last_bank,
            _ => self.prg_bank as usize,
        };
        self.cartridge.prg_rom.read_mapped(bank, 16 * 1024, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr & 0xd000 {
            // 0x8000 - 0x9fff is the range for this reg, but it only exists on FireHawk and that game just writes to 0x9000 - 0x9fff
            // this if statement lets us hackily support FireHawk without caring about the submapper
            0x9000 => {
                if value & 0x10 != 0 {
                    self.mirroring.internal_a();
                } else {
                    self.mirroring.internal_b();
                }
            }
            0xc000 | 0xd000 => self.prg_bank = value & 0xf,
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if let Some(chr_ram) = self.chr_ram.as_ref() {
            chr_ram.read_mapped(0, 8 * 1024, addr)
        } else {
            self.cartridge.chr_rom.read_mapped(0, 8 * 1024, addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if let Some(chr_ram) = self.chr_ram.as_mut() {
            chr_ram.write_mapped(0, 8 * 1024, addr, value)
        }
    }
}

impl Mapper for Bf909x {
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
