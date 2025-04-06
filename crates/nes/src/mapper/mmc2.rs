#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
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
    prg_ram: Option<MemoryBlock>,
    prg_bank: u8,
    chr_banks: [u8; 4],
    chr_latches: [u8; 2],
    mirroring: SimpleMirroring,
}

impl Mmc2 {
    pub fn new(mut cartridge: INes, variant: Mmc2Variant) -> Self {
        let prg_ram = if cartridge.prg_ram_bytes > 0 {
            let mut ram = MemoryBlock::new(8);
            if let Some(wram) = cartridge.wram.take() {
                ram.restore_wram(wram);
            }
            Some(ram)
        } else {
            None
        };

        let prg_bank_count = cartridge.prg_rom.len() / 0x2000;

        Self {
            mirroring: SimpleMirroring::new(cartridge.mirroring),
            variant,
            cartridge,
            prg_ram,
            prg_bank_count,
            prg_bank: 0,
            chr_banks: [0; 4],
            chr_latches: [0xfd; 2],
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0x8000 == 0 {
            if let Some(ram) = self.prg_ram.as_ref() {
                ram.read_mapped(0, 8 * 1024, addr)
            } else {
                0
            }
        } else {
            let (bank, size) = self.map_prg(addr);
            self.cartridge.prg_rom.read_mapped(bank, size, addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0xa000 => self.prg_bank = value & 0xf,
            0xb000 => self.chr_banks[0] = value & 0x1f,
            0xc000 => self.chr_banks[1] = value & 0x1f,
            0xd000 => self.chr_banks[2] = value & 0x1f,
            0xe000 => self.chr_banks[3] = value & 0x1f,
            0xf000 if value & 1 == 0 => self.mirroring.vertical(),
            0xf000 if value & 1 == 1 => self.mirroring.horizontal(),
            0x6000..=0x7fff => {
                if let Some(ram) = self.prg_ram.as_mut() {
                    ram.write_mapped(0, 8 * 1024, addr, value)
                }
            }
            _ => return,
        }
    }

    fn peek_ppu(&self, addr: u16) -> u8 {
        let latch_idx = if addr & 0x1000 == 0 { 0 } else { 1 };
        let latch = self.chr_latches[latch_idx];
        let bank_idx = if latch == 0xfd {
            0 | latch_idx << 1
        } else {
            1 | latch_idx << 1
        };
        let bank = self.chr_banks[bank_idx];
        self.cartridge
            .chr_rom
            .read_mapped(bank as usize, 4 * 1024, addr)
    }

    fn read_ppu(&mut self, addr: u16) -> u8 {
        let val = self.peek_ppu(addr);

        match addr {
            0x0fd8 => self.chr_latches[0] = 0xfd,
            0x0fe8 => self.chr_latches[0] = 0xfe,
            0x0fd8..=0x0fdf if self.variant.is_mmc4() => self.chr_latches[0] = 0xfd,
            0x0fe8..=0x0fef if self.variant.is_mmc4() => self.chr_latches[0] = 0xfe,
            0x1fd8..=0x1fdf => self.chr_latches[1] = 0xfd,
            0x1fe8..=0x1fef => self.chr_latches[1] = 0xfe,
            _ => (),
        }

        val
    }

    fn map_prg(&self, addr: u16) -> (usize, usize) {
        let (bank, size) = if self.variant.is_mmc4() {
            match addr & 0x4000 {
                0 => (self.prg_bank as usize, 16),
                _ => ((self.prg_bank_count / 2) - 1, 16),
            }
        } else {
            match addr & 0xe000 {
                0x8000 => (self.prg_bank as usize, 8),
                0xa000 => (self.prg_bank_count - 3, 8),
                0xc000 => (self.prg_bank_count - 2, 8),
                0xe000 => (self.prg_bank_count - 1, 8),
                _ => unreachable!(),
            }
        };

        (bank, size * 1024)
    }
}

impl Mapper for Mmc2 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xf000));

        if self.prg_ram.is_some() {
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

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.as_ref().and_then(|r| r.save_wram())
        } else {
            None
        }
    }
}
