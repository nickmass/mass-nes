use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, Cartridge};
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};

use std::cell::RefCell;

use super::{Nametable, SimpleMirroring};

pub struct TxromState {
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
    last_a12: bool,
    last: usize,
    ext_nt: Option<[MemoryBlock; 2]>,
}

impl TxromState {
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

pub struct Txrom {
    cartridge: Cartridge,
    state: RefCell<TxromState>,
    mirroring: SimpleMirroring,
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

        let mut rom_state = TxromState {
            current_tick: 0,
            last_a12_tick: 0,
            prg,
            chr,
            chr_type,
            bank_data: [0; 8],
            bank_select: 0,
            ram_protect: false,
            ram_enabled: false,
            irq: false,
            irq_enabled: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
            last_a12: false,
            ext_nt,
            last: (cartridge.prg_rom.len() / 0x2000) - 1,
        };

        rom_state.sync();

        Txrom {
            state: RefCell::new(rom_state),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        let rom = self.state.borrow();
        if addr & 0xe000 == 0x6000 && !rom.ram_enabled {
            (addr & 0xff) as u8
        } else {
            self.state.borrow().prg.read(&self.cartridge, addr)
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.irq_tick(addr);
        self.peek_ppu(addr)
    }

    fn peek_ppu(&self, addr: u16) -> u8 {
        let rom = self.state.borrow();
        if let Some([a, b]) = rom.ext_nt.as_ref() {
            if addr & 0x2000 != 0 {
                match addr & 0x400 {
                    0x0000 => a.read(addr & 0x3ff),
                    0x0400 => b.read(addr & 0x3ff),
                    _ => unreachable!(),
                }
            } else {
                rom.chr.read(&self.cartridge, addr)
            }
        } else {
            rom.chr.read(&self.cartridge, addr)
        }
    }

    fn write_cpu(&self, addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        if addr & 0xe000 == 0x6000 {
            if rom.ram_enabled && !rom.ram_protect {
                rom.prg.write(addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                rom.bank_select = value;
                rom.sync();
            }
            0x8001 => {
                let bank_index = rom.bank_select & 0x7;
                rom.bank_data[bank_index as usize] = value;
                rom.sync();
            }
            0xa000 => {
                if rom.ext_nt.is_some() {
                    return;
                }

                match value & 1 {
                    0 => self.mirroring.vertical(),
                    1 => self.mirroring.horizontal(),
                    _ => unreachable!(),
                }
            }
            0xa001 => {
                rom.ram_protect = value & 0x40 != 0;
                rom.ram_enabled = value & 0x80 != 0;
            }
            0xc000 => {
                rom.irq_latch = value;
            }
            0xc001 => {
                rom.irq_reload_pending = true;
            }
            0xe000 => {
                rom.irq = false;
                rom.irq_enabled = false;
            }
            0xe001 => {
                rom.irq_enabled = true;
            }
            _ => unreachable!(),
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        self.irq_tick(addr);
        let mut rom = self.state.borrow_mut();
        if let Some([a, b]) = rom.ext_nt.as_mut() {
            if addr & 0x2000 != 0 {
                match addr & 0x400 {
                    0x0000 => a.write(addr & 0x3ff, value),
                    0x0400 => b.write(addr & 0x3ff, value),
                    _ => unreachable!(),
                }
            } else {
                rom.chr.write(addr, value);
            }
        } else {
            rom.chr.write(addr, value);
        }
    }

    fn irq_tick(&self, addr: u16) {
        let mut rom = self.state.borrow_mut();
        let a12 = addr & 0x1000 != 0;
        let mut clock = !rom.last_a12 && a12;
        if clock {
            if rom.current_tick - rom.last_a12_tick < 5 {
                clock = false
            }
            rom.last_a12_tick = rom.current_tick;
        }
        rom.last_a12 = a12;
        let mut is_zero = false;
        if clock {
            if rom.irq_reload_pending {
                rom.irq_counter = rom.irq_latch;
                rom.irq_reload_pending = false;
                if rom.irq_counter == 0 {
                    is_zero = true;
                }
            } else {
                rom.irq_counter = rom.irq_counter.saturating_sub(1);
                if rom.irq_counter == 0 {
                    is_zero = true;
                    rom.irq_reload_pending = true;
                }
            }
            if is_zero && rom.irq_enabled {
                rom.irq = true;
            }
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
            BusKind::Ppu => self.peek_ppu(addr),
        }
    }

    fn read(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn write(&self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn tick(&self) {
        let mut rom = self.state.borrow_mut();
        rom.current_tick += 1;
    }

    fn get_irq(&self) -> bool {
        let rom = self.state.borrow();
        rom.irq
    }

    fn update_ppu_addr(&self, addr: u16) {
        self.irq_tick(addr);
    }

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        let rom = self.state.borrow();
        if let Some(_) = rom.ext_nt {
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
}