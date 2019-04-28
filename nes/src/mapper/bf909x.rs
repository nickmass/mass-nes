use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, NotAndMask};
use crate::cartridge::{Cartridge, Mirroring};
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use crate::nametables::Nametable;
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

use std::cell::RefCell;

pub struct Bf909xState {
    mem: MappedMemory,
}

pub struct Bf909x {
    chr_ram: MemoryBlock,
    state: RefCell<Bf909xState>,
}

impl Bf909x {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Bf909x {
        let mut rom_state = Bf909xState {
            mem: MappedMemory::new(state, cartridge, 0x8000, 0, 32, MemKind::Prg),
        };
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        rom_state.mem.map(0xC000, 16, last, BankKind::Rom);
        Bf909x {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10, &mut state.mem),
            state: RefCell::new(rom_state),
        }
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().mem.read(system, state, addr)
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        if system.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.read(state, addr)
        } else {
            system.cartridge.chr_rom[addr as usize]
        }
    }

    fn write_cpu(&self, system: &System, sys_state: &mut SystemState, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        match addr & 0xd000 {
            // 0x8000 - 0x9fff is the range for this reg, but it only exists on FireHawk and that game just writes to 0x9000 - 0x9fff
            // this if statement lets us hackily support FireHawk without caring about the submapper
            0x9000 => {
                if value & 0x10 != 0 {
                    system
                        .ppu
                        .nametables
                        .set_single(sys_state, Nametable::First);
                } else {
                    system
                        .ppu
                        .nametables
                        .set_single(sys_state, Nametable::Second);
                }
            }
            0xc000 | 0xd000 => state
                .mem
                .map(0x8000, 16, (value & 0xf) as usize, BankKind::Rom),
            _ => (),
        }
    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        if system.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(state, addr, value);
        }
    }
}

impl Mapper for Bf909x {
    fn register(
        &self,
        state: &mut SystemState,
        cpu: &mut AddressBus,
        ppu: &mut Ppu,
        cart: &Cartridge,
    ) {
        cpu.register_read(
            state,
            DeviceKind::Mapper,
            AndAndMask(0x8000, (cart.prg_rom.len() - 1) as u16),
        );
        cpu.register_write(
            state,
            DeviceKind::Mapper,
            AndAndMask(0x8000, (cart.prg_rom.len() - 1) as u16),
        );
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        ppu.register_write(state, DeviceKind::Mapper, NotAndMask(0x1fff));
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
            BusKind::Ppu => self.write_ppu(system, state, addr, value),
        }
    }

    fn tick(&self, system: &System, state: &mut SystemState) {}

    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        system.ppu.nametables.write(state, addr, value);
    }

    fn update_ppu_addr(&self, system: &System, state: &mut SystemState, addr: u16) {}
}
