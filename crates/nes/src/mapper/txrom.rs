#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, Cartridge};
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};

use super::{Nametable, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Txrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: Cartridge,
    mirroring: SimpleMirroring,
    current_tick: u64,
    last_a12_tick: u64,
    prg: MappedMemory,
    chr: MappedMemory,
    chr_type: BankKind,
    bank_data: [u8; 8],
    bank_select: u8,
    ram_protect: bool,
    ram_enabled: bool,
    irq: bool,
    irq_enabled: bool,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload_pending: bool,
    last: usize,
    ext_nt: Option<[MemoryBlock; 2]>,
}

impl Txrom {
    pub fn new(cartridge: Cartridge) -> Txrom {
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

        let mut prg = MappedMemory::new(&cartridge, 0x6000, 16, 48, MemKind::Prg);
        prg.map(0x6000, 16, 0, BankKind::Ram);

        let ext_nt = if cartridge.mirroring == CartMirroring::FourScreen {
            Some([MemoryBlock::new(1), MemoryBlock::new(1)])
        } else {
            None
        };

        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());
        let last = (cartridge.prg_rom.len() / 0x2000) - 1;

        let mut rom = Txrom {
            cartridge,
            mirroring,
            current_tick: 0,
            last_a12_tick: 0,
            prg,
            chr,
            chr_type,
            bank_data: [0; 8],
            bank_select: 0,
            ram_protect: false,
            ram_enabled: true,
            irq: false,
            irq_enabled: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
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
                self.irq_reload_pending = true;
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
        self.irq_tick(addr);
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

    fn irq_tick(&mut self, addr: u16) {
        let a12 = addr & 0x1000 != 0;
        let mut clock = a12;
        if clock {
            if self.current_tick - self.last_a12_tick <= 3 {
                clock = false
            }
            self.last_a12_tick = self.current_tick;
        }
        let mut is_zero = false;
        if clock {
            if self.irq_reload_pending {
                self.irq_counter = self.irq_latch;
                self.irq_reload_pending = false;
                if self.irq_counter == 0 {
                    is_zero = true;
                }
            } else {
                self.irq_counter = self.irq_counter.saturating_sub(1);
                if self.irq_counter == 0 {
                    is_zero = true;
                    self.irq_reload_pending = true;
                }
            }
            if is_zero && self.irq_enabled {
                self.irq = true;
            }
        }
    }

    fn sync(&mut self) {
        if self.chr_type == BankKind::Rom {
            if self.bank_select & 0x80 == 0 {
                self.chr.map(
                    0x0000,
                    1,
                    (self.bank_data[0] & 0xfe) as usize,
                    BankKind::Rom,
                );
                self.chr
                    .map(0x0400, 1, (self.bank_data[0] | 0x1) as usize, BankKind::Rom);
                self.chr.map(
                    0x0800,
                    1,
                    (self.bank_data[1] & 0xfe) as usize,
                    BankKind::Rom,
                );
                self.chr.map(
                    0x0c00,
                    1,
                    (self.bank_data[1] | 0x01) as usize,
                    BankKind::Rom,
                );
                self.chr
                    .map(0x1000, 1, self.bank_data[2] as usize, BankKind::Rom);
                self.chr
                    .map(0x1400, 1, self.bank_data[3] as usize, BankKind::Rom);
                self.chr
                    .map(0x1800, 1, self.bank_data[4] as usize, BankKind::Rom);
                self.chr
                    .map(0x1c00, 1, self.bank_data[5] as usize, BankKind::Rom);
            } else {
                self.chr
                    .map(0x0000, 1, self.bank_data[2] as usize, BankKind::Rom);
                self.chr
                    .map(0x0400, 1, self.bank_data[3] as usize, BankKind::Rom);
                self.chr
                    .map(0x0800, 1, self.bank_data[4] as usize, BankKind::Rom);
                self.chr
                    .map(0x0c00, 1, self.bank_data[5] as usize, BankKind::Rom);
                self.chr.map(
                    0x1000,
                    1,
                    (self.bank_data[0] & 0xfe) as usize,
                    BankKind::Rom,
                );
                self.chr
                    .map(0x1400, 1, (self.bank_data[0] | 0x1) as usize, BankKind::Rom);
                self.chr.map(
                    0x1800,
                    1,
                    (self.bank_data[1] & 0xfe) as usize,
                    BankKind::Rom,
                );
                self.chr.map(
                    0x1c00,
                    1,
                    (self.bank_data[1] | 0x01) as usize,
                    BankKind::Rom,
                );
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

impl Mapper for Txrom {
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
        self.current_tick += 1;
    }

    fn get_irq(&mut self) -> bool {
        self.irq
    }

    fn peek_ppu_fetch(&self, address: u16) -> Nametable {
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

    fn ppu_fetch(&mut self, address: u16) -> super::Nametable {
        self.irq_tick(address);
        self.peek_ppu_fetch(address)
    }
}
