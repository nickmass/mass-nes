use nes::system::{System, SystemState};
use nes::memory::{BankKind, MappedMemory, MemKind};
use nes::bus::{DeviceKind, BusKind, AndAndMask, NotAndMask};
use nes::cartridge::{Mirroring, Cartridge};
use nes::cpu::Cpu;
use nes::ppu::Ppu;
use nes::mapper::Mapper;

use std::cell::RefCell;

pub struct CnromState {
    chr: MappedMemory, 
}

pub struct Cnrom {
    state: RefCell<CnromState>,
}

impl Cnrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Cnrom {
        let rom_state = CnromState {
            chr : MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr),
        };
        Cnrom {
            state: RefCell::new(rom_state),
        }
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.cartridge.prg_rom[addr as usize]
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().chr.read(system, state, addr)
    }

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        state.chr.map(0x0000, 8, (value)as usize, BankKind::Rom);
    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
    }
}

impl Mapper for Cnrom {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
    cart: &Cartridge) {
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000,
                                        (cart.prg_rom.len() - 1) as u16));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000,
                                        (cart.prg_rom.len() - 1) as u16));
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        ppu.register_write(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        match cart.mirroring {
            Mirroring::Horizontal => ppu.nametables.set_horizontal(state),
            Mirroring::Vertical => ppu.nametables.set_vertical(state),
            Mirroring::FourScreen => {
                unimplemented!()
            }
        }
    }

    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr:u16)
    -> u8 {
        match bus {
            BusKind::Cpu => {
                self.read_cpu(system, state, addr)
            },
            BusKind::Ppu => {
                self.read_ppu(system, state, addr)
            },
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16)
    -> u8 {
        match bus {
            BusKind::Cpu => {
                self.read_cpu(system, state, addr)
            },
            BusKind::Ppu => {
                self.read_ppu(system, state, addr)
            },
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState,
    addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                self.write_cpu(system, state, addr, value)
            },
            BusKind::Ppu => {
                self.write_ppu(system, state, addr, value)
            },
        }
    }

    fn tick(&self, system: &System, state: &mut SystemState) {}
}
