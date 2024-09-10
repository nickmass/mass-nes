use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, NotAndMask};
use crate::cartridge::{Cartridge, Mirroring};
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

use std::cell::RefCell;

pub struct PxromState {
    prg_bank_count: usize,
    chr_bank_count: usize,
    prg: MappedMemory,
    chr: MappedMemory,
    prg_bank: u8,
    chr_banks: [u8; 4],
    chr_latches: [u8; 2],
}

impl PxromState {
    fn sync(&mut self) {
        self.prg.map(
            0x8000,
            8,
            (self.prg_bank & 0xf) as usize % self.prg_bank_count,
            BankKind::Rom,
        );
        self.prg
            .map(0xa000, 8, self.prg_bank_count - 3, BankKind::Rom);
        self.prg
            .map(0xc000, 8, self.prg_bank_count - 2, BankKind::Rom);
        self.prg
            .map(0xe000, 8, self.prg_bank_count - 1, BankKind::Rom);

        let lo = if self.chr_latches[0] == 0xfd { 0 } else { 1 };
        let hi = if self.chr_latches[1] == 0xfd { 2 } else { 3 };

        self.chr.map(
            0x0000,
            4,
            (self.chr_banks[lo] & 0x1f) as usize % self.chr_bank_count,
            BankKind::Rom,
        );
        self.chr.map(
            0x1000,
            4,
            (self.chr_banks[hi] & 0x1f) as usize % self.chr_bank_count,
            BankKind::Rom,
        );
    }
}

pub struct Pxrom {
    state: RefCell<PxromState>,
}

impl Pxrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Self {
        let mut rom_state = PxromState {
            prg_bank_count: cartridge.prg_rom.len() / 0x2000,
            chr_bank_count: cartridge.chr_rom.len() / 0x1000,
            prg: MappedMemory::new(state, cartridge, 0x8000, 0, 32, MemKind::Prg),
            chr: MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr),
            prg_bank: 0,
            chr_banks: [0; 4],
            chr_latches: [0xfd; 2],
        };

        rom_state.sync();

        Self {
            state: RefCell::new(rom_state),
        }
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().prg.read(system, state, addr)
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        let mut rom = self.state.borrow_mut();
        let val = rom.chr.read(system, state, addr);

        match addr {
            0x0fd8 => rom.chr_latches[0] = 0xfd,
            0x0fe8 => rom.chr_latches[0] = 0xfe,
            0x1fd8..=0x1fdf => rom.chr_latches[1] = 0xfd,
            0x1fe8..=0x1fef => rom.chr_latches[1] = 0xfe,
            _ => return val,
        }

        rom.sync();
        val
    }

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        match addr {
            0xa000 => rom.prg_bank = value,
            0xb000 => rom.chr_banks[0] = value,
            0xc000 => rom.chr_banks[1] = value,
            0xd000 => rom.chr_banks[2] = value,
            0xe000 => rom.chr_banks[3] = value,
            0xf000 if value & 1 == 0 => system.ppu.nametables.set_vertical(state),
            0xf000 if value & 1 == 1 => system.ppu.nametables.set_horizontal(state),
            _ => return,
        }

        rom.sync();
    }
}

impl Mapper for Pxrom {
    fn register(
        &self,
        state: &mut SystemState,
        cpu: &mut AddressBus,
        ppu: &mut Ppu,
        cart: &Cartridge,
    ) {
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xf000));
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        match cart.mirroring {
            Mirroring::Horizontal => ppu.nametables.set_horizontal(state),
            Mirroring::Vertical => ppu.nametables.set_vertical(state),
            Mirroring::FourScreen => unimplemented!(),
        }
    }

    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(system, state, addr),
            BusKind::Ppu => self.read_ppu(system, state, addr),
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(system, state, addr),
            BusKind::Ppu => self.read_ppu(system, state, addr),
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(system, state, addr, value),
            BusKind::Ppu => (),
        }
    }
}
