use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::{Nametable, SimpleMirroring};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PrgRamState {
    ReadWrite,
    ReadOnly,
    Zero,
    OpenBus,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum Mmc3Variant {
    Mmc3,
    Mmc3AltIrq,
    Mmc6,
}

impl Mmc3Variant {
    fn is_mmc6(&self) -> bool {
        matches!(self, Mmc3Variant::Mmc6)
    }

    fn is_alt_irq(&self) -> bool {
        matches!(self, Mmc3Variant::Mmc3AltIrq)
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc3 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    variant: Mmc3Variant,
    mirroring: SimpleMirroring,
    prg_ram: MemoryBlock,
    chr_ram: Option<FixedMemoryBlock<8>>,
    bank_data: [u8; 8],
    bank_select: u8,
    ram_enabled: bool,
    ram_reg: u8,
    irq: bool,
    irq_enabled: bool,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload_pending: bool,
    irq_force_reload_pending: bool,
    irq_a12: bool,
    irq_a12_low_cycles: u64,
    last_prg: u8,
    ext_nt: Option<FixedMemoryBlock<2>>,
}

impl Mmc3 {
    pub fn new(mut cartridge: INes, variant: Mmc3Variant, debug: Rc<Debug>) -> Mmc3 {
        let mut prg_ram = if variant.is_mmc6() {
            MemoryBlock::new(1)
        } else {
            MemoryBlock::new(8)
        };

        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }

        let chr_ram = cartridge
            .chr_rom
            .is_empty()
            .then(|| FixedMemoryBlock::new());

        let (mirroring, ext_nt) = if cartridge.alternative_mirroring {
            let mirroring = SimpleMirroring::new(super::Mirroring::FourScreen);
            (mirroring, Some(FixedMemoryBlock::new()))
        } else {
            (SimpleMirroring::new(cartridge.mirroring), None)
        };

        let last_prg = (cartridge.prg_rom.len() / 0x2000 - 1) as u8;

        Self {
            cartridge,
            debug,
            variant,
            mirroring,
            prg_ram,
            chr_ram,
            bank_data: [0; 8],
            bank_select: 0,
            ram_enabled: true,
            ram_reg: 0,
            irq: false,
            irq_enabled: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
            irq_force_reload_pending: false,
            irq_a12: false,
            irq_a12_low_cycles: 0,
            ext_nt,
            last_prg,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0xe000 == 0x6000 {
            match self.prg_ram_state(addr) {
                PrgRamState::ReadWrite | PrgRamState::ReadOnly => {
                    if self.variant.is_mmc6() {
                        self.prg_ram.read_mapped(0, 1024, addr)
                    } else {
                        self.prg_ram.read_mapped(0, 8 * 1024, addr)
                    }
                }
                PrgRamState::Zero => 0,
                PrgRamState::OpenBus => addr as u8,
            }
        } else {
            self.read_prg(addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr & 0xe000 == 0x6000 {
            if self.prg_ram_state(addr) == PrgRamState::ReadWrite {
                if self.variant.is_mmc6() {
                    self.prg_ram.write_mapped(0, 1024, addr, value)
                } else {
                    self.prg_ram.write_mapped(0, 8 * 1024, addr, value)
                }
            }
            return;
        }

        match addr {
            0x8000 => {
                self.bank_select = value;
                if self.variant.is_mmc6() {
                    self.ram_enabled = value & 0x20 != 0;
                }
            }
            0x8001 => {
                let bank_index = self.bank_select & 0x7;
                self.bank_data[bank_index as usize] = value;
            }
            0xa000 => {
                if self.ext_nt.is_some() {
                    return;
                } else if value & 1 == 0 {
                    self.mirroring.vertical()
                } else {
                    self.mirroring.horizontal()
                }
            }
            0xa001 => {
                if !self.variant.is_mmc6() {
                    self.ram_enabled = value & 0x80 != 0;
                }
                self.ram_reg = value;
            }
            0xc000 => self.irq_latch = value,
            0xc001 => self.irq_force_reload_pending = true,
            0xe000 => {
                self.irq = false;
                self.irq_enabled = false;
            }
            0xe001 => self.irq_enabled = true,
            _ => unreachable!(),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 != 0 {
            if let Some(nt) = self.ext_nt.as_ref() {
                nt.read(addr)
            } else {
                0
            }
        } else {
            let (bank, size) = self.map_chr(addr);
            if let Some(ram) = self.chr_ram.as_ref() {
                ram.read_mapped(bank, size, addr)
            } else {
                self.cartridge.chr_rom.read_mapped(bank, size, addr)
            }
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr & 0x2000 != 0 {
            if let Some(nt) = self.ext_nt.as_mut() {
                nt.write(addr, value)
            }
        } else {
            let (bank, size) = self.map_chr(addr);
            if let Some(ram) = self.chr_ram.as_mut() {
                ram.write_mapped(bank, size, addr, value)
            }
        }
    }

    fn map_chr(&self, addr: u16) -> (usize, usize) {
        let (bank, size) = match &self.chr_ram {
            Some(_) => (0, 8),
            None if self.bank_select & 0x80 == 0 => match addr >> 10 & 7 {
                0 | 1 => (self.bank_data[0] >> 1, 2),
                2 | 3 => (self.bank_data[1] >> 1, 2),
                n => (self.bank_data[n as usize - 2], 1),
            },
            None => match addr >> 10 & 7 {
                4 | 5 => (self.bank_data[0] >> 1, 2),
                6 | 7 => (self.bank_data[1] >> 1, 2),
                n => (self.bank_data[n as usize + 2], 1),
            },
        };

        (bank as usize, size * 1024)
    }

    fn read_prg(&self, addr: u16) -> u8 {
        let block = addr >> 13 & 3;
        let bank = if self.bank_select & 0x40 == 0 {
            match block {
                0 => self.bank_data[6],
                1 => self.bank_data[7],
                2 => self.last_prg - 1,
                3 => self.last_prg,
                _ => unreachable!(),
            }
        } else {
            match block {
                0 => self.last_prg - 1,
                1 => self.bank_data[7],
                2 => self.bank_data[6],
                3 => self.last_prg,
                _ => unreachable!(),
            }
        };

        self.cartridge
            .prg_rom
            .read_mapped(bank as usize, 8 * 1024, addr)
    }

    fn prg_ram_state(&self, addr: u16) -> PrgRamState {
        if !self.ram_enabled {
            PrgRamState::OpenBus
        } else if !self.variant.is_mmc6() {
            if self.ram_reg & 0x40 == 0 {
                PrgRamState::ReadWrite
            } else {
                PrgRamState::ReadOnly
            }
        } else {
            match addr & 0x7200 {
                0x7000 if self.ram_reg & 0x30 == 0x30 => PrgRamState::ReadWrite,
                0x7200 if self.ram_reg & 0xc0 == 0xc0 => PrgRamState::ReadWrite,
                0x7000 if self.ram_reg & 0x20 != 0 => PrgRamState::ReadOnly,
                0x7200 if self.ram_reg & 0x80 != 0 => PrgRamState::ReadOnly,
                _ if self.ram_reg & 0xa0 != 0 => PrgRamState::Zero,
                _ => PrgRamState::OpenBus,
            }
        }
    }

    fn irq_addr(&mut self, addr: u16) {
        let a12 = addr & 0x1000 != 0;
        let clock = a12 && !self.irq_a12 && self.irq_a12_low_cycles > 3;
        if a12 {
            self.irq_a12_low_cycles = 0
        }
        self.irq_a12 = a12;

        if clock {
            let was_zero = self.variant.is_alt_irq()
                && self.irq_counter == 0
                && !self.irq_force_reload_pending;
            if self.irq_reload_pending || self.irq_force_reload_pending || self.irq_counter == 0 {
                self.irq_counter = self.irq_latch;
                self.irq_reload_pending = false;
                self.irq_force_reload_pending = false;
            } else {
                self.irq_counter = self.irq_counter.saturating_sub(1);
                if self.irq_counter == 0 {
                    self.irq_reload_pending = true;
                }
            }
            if self.irq_counter == 0 && self.irq_enabled && !was_zero {
                if !self.irq {
                    self.debug.event(crate::DebugEvent::MapperIrq);
                }
                self.irq = true;
            }
        }
    }
}

impl Mapper for Mmc3 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xe001));
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

    fn tick(&mut self) {
        if self.irq_a12 {
            self.irq_a12_low_cycles = 0;
        } else {
            self.irq_a12_low_cycles += 1;
        }
    }

    fn get_irq(&mut self) -> bool {
        self.irq
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn ppu_fetch(&mut self, address: u16, kind: PpuFetchKind) -> super::Nametable {
        self.irq_addr(address);
        self.peek_ppu_fetch(address, kind)
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.save_wram()
        } else {
            None
        }
    }
}
