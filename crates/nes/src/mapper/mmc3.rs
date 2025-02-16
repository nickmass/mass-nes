use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, INes};
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::{Nametable, SimpleMirroring};

const MMC3_ALT_IRQ_BEHAVIOR: bool = false;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc3 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    mirroring: SimpleMirroring,
    prg: MappedMemory,
    chr: MappedMemory,
    chr_type: BankKind,
    chr_count: usize,
    bank_data: [u8; 8],
    bank_select: u8,
    ram_protect: bool,
    ram_enabled: bool,
    irq: bool,
    irq_enabled: bool,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload_pending: bool,
    irq_force_reload_pending: bool,
    irq_a12: bool,
    irq_a12_low_cycles: u64,
    last: usize,
    ext_nt: Option<[MemoryBlock; 2]>,
}

impl Mmc3 {
    pub fn new(mut cartridge: INes, debug: Rc<Debug>) -> Mmc3 {
        let chr_type = if cartridge.chr_rom.is_empty() {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let chr = match chr_type {
            BankKind::Rom => MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram => {
                let mut mem = MappedMemory::new(&cartridge, 0x0000, 8, 8, MemKind::Chr);
                mem.map(0x0000, 8, 0, BankKind::Ram);
                mem
            }
        };

        let chr_count = match chr_type {
            BankKind::Ram => 0,
            BankKind::Rom => cartridge.chr_rom.len() / 1024,
        };

        let mut prg = MappedMemory::new(&cartridge, 0x6000, 16, 48, MemKind::Prg);
        prg.map(0x6000, 16, 0, BankKind::Ram);

        if let Some(wram) = cartridge.wram.take() {
            prg.restore_wram(wram);
        }

        let ext_nt = if cartridge.mirroring == CartMirroring::FourScreen {
            Some([MemoryBlock::new(1), MemoryBlock::new(1)])
        } else {
            None
        };

        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());
        let last = (cartridge.prg_rom.len() / 0x2000) - 1;

        let mut rom = Mmc3 {
            cartridge,
            debug,
            mirroring,
            prg,
            chr,
            chr_type,
            chr_count,
            bank_data: [0; 8],
            bank_select: 0,
            ram_protect: false,
            ram_enabled: true,
            irq: false,
            irq_enabled: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
            irq_force_reload_pending: false,
            irq_a12: false,
            irq_a12_low_cycles: 0,
            ext_nt,
            last,
        };

        rom.sync();

        rom
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0xe000 == 0x6000 && !self.ram_enabled {
            (addr & 0xff) as u8
        } else {
            self.prg.read(&self.cartridge, addr)
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if let Some([a, b]) = self.ext_nt.as_ref() {
            if addr & 0x2000 != 0 {
                match addr & 0x400 {
                    0x0000 => a.read(addr & 0x3ff),
                    0x0400 => b.read(addr & 0x3ff),
                    _ => unreachable!(),
                }
            } else {
                self.chr.read(&self.cartridge, addr)
            }
        } else {
            self.chr.read(&self.cartridge, addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr & 0xe000 == 0x6000 {
            if self.ram_enabled && !self.ram_protect {
                self.prg.write(addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                self.bank_select = value;
                self.sync();
            }
            0x8001 => {
                let bank_index = self.bank_select & 0x7;
                self.bank_data[bank_index as usize] = value;
                self.sync();
            }
            0xa000 => {
                if self.ext_nt.is_some() {
                    return;
                }

                match value & 1 {
                    0 => self.mirroring.vertical(),
                    1 => self.mirroring.horizontal(),
                    _ => unreachable!(),
                }
            }
            0xa001 => {
                self.ram_protect = value & 0x40 != 0;
                self.ram_enabled = value & 0x80 != 0;
            }
            0xc000 => {
                self.irq_latch = value;
            }
            0xc001 => {
                self.irq_force_reload_pending = true;
            }
            0xe000 => {
                self.irq = false;
                self.irq_enabled = false;
            }
            0xe001 => {
                self.irq_enabled = true;
            }
            _ => unreachable!(),
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if let Some([a, b]) = self.ext_nt.as_mut() {
            if addr & 0x2000 != 0 {
                match addr & 0x400 {
                    0x0000 => a.write(addr & 0x3ff, value),
                    0x0400 => b.write(addr & 0x3ff, value),
                    _ => unreachable!(),
                }
            } else {
                self.chr.write(addr, value);
            }
        } else {
            self.chr.write(addr, value);
        }
    }

    fn irq_addr(&mut self, addr: u16) {
        let a12 = addr & 0x1000 != 0;
        let clock = a12 && !self.irq_a12 && self.irq_a12_low_cycles > 3;
        if a12 {
            self.irq_a12_low_cycles = 0
        }
        self.irq_a12 = a12;

        let mut is_zero = false;
        if clock {
            let was_zero =
                MMC3_ALT_IRQ_BEHAVIOR && self.irq_counter == 0 && !self.irq_force_reload_pending;
            if self.irq_reload_pending || self.irq_force_reload_pending {
                self.irq_counter = self.irq_latch;
                if self.irq_counter == 0 {
                    is_zero = true;
                }
                self.irq_reload_pending = false;
                self.irq_force_reload_pending = false;
            } else {
                self.irq_counter = self.irq_counter.saturating_sub(1);
                if self.irq_counter == 0 {
                    is_zero = true;
                    self.irq_reload_pending = true;
                }
            }
            if is_zero && self.irq_enabled && !was_zero {
                if !self.irq {
                    self.debug.event(crate::DebugEvent::MapperIrq);
                }
                self.irq = true;
            }
        }
    }

    fn sync(&mut self) {
        if self.chr_type == BankKind::Rom {
            let chr = |n| self.bank_data[n] as usize % self.chr_count;
            if self.bank_select & 0x80 == 0 {
                self.chr.map(0x0000, 2, chr(0) >> 1, BankKind::Rom);
                self.chr.map(0x0800, 2, chr(1) >> 1, BankKind::Rom);
                self.chr.map(0x1000, 1, chr(2), BankKind::Rom);
                self.chr.map(0x1400, 1, chr(3), BankKind::Rom);
                self.chr.map(0x1800, 1, chr(4), BankKind::Rom);
                self.chr.map(0x1c00, 1, chr(5), BankKind::Rom);
            } else {
                self.chr.map(0x0000, 1, chr(2), BankKind::Rom);
                self.chr.map(0x0400, 1, chr(3), BankKind::Rom);
                self.chr.map(0x0800, 1, chr(4), BankKind::Rom);
                self.chr.map(0x0c00, 1, chr(5), BankKind::Rom);
                self.chr.map(0x1000, 2, chr(0) >> 1, BankKind::Rom);
                self.chr.map(0x1800, 2, chr(1) >> 1, BankKind::Rom);
            }
        }

        if self.bank_select & 0x40 == 0 {
            self.prg
                .map(0x8000, 8, self.bank_data[6] as usize, BankKind::Rom);
            self.prg
                .map(0xa000, 8, self.bank_data[7] as usize, BankKind::Rom);
            self.prg
                .map(0xc000, 8, (self.last - 1) as usize, BankKind::Rom);
            self.prg.map(0xe000, 8, self.last as usize, BankKind::Rom);
        } else {
            self.prg
                .map(0x8000, 8, (self.last - 1) as usize, BankKind::Rom);
            self.prg
                .map(0xa000, 8, self.bank_data[7] as usize, BankKind::Rom);
            self.prg
                .map(0xc000, 8, self.bank_data[6] as usize, BankKind::Rom);
            self.prg.map(0xe000, 8, self.last as usize, BankKind::Rom);
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

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
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
        if let Some(_) = self.ext_nt {
            if address & 0x2000 != 0 {
                match address & 0xc00 {
                    0x0000 => Nametable::InternalA,
                    0x0400 => Nametable::InternalB,
                    0x0800 | 0xc00 => Nametable::External,
                    _ => unreachable!(),
                }
            } else {
                Nametable::External
            }
        } else {
            self.mirroring.ppu_fetch(address)
        }
    }

    fn ppu_fetch(&mut self, address: u16, kind: PpuFetchKind) -> super::Nametable {
        self.irq_addr(address);
        self.peek_ppu_fetch(address, kind)
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg.save_wram()
        } else {
            None
        }
    }
}
