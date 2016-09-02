use nes::system::{System, SystemState};
use nes::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use nes::bus::{DeviceKind, BusKind, AndAndMask, NotAndMask};
use nes::cartridge::{Mirroring, Cartridge};
use nes::cpu::Cpu;
use nes::ppu::Ppu;
use nes::mapper::Mapper;

use std::cell::RefCell;

pub struct UxromState {
    mem: MappedMemory, 
}

pub struct Uxrom {
    chr_ram: MemoryBlock,
    state: RefCell<UxromState>,
}

impl Uxrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Uxrom {
        let mut rom_state = UxromState {
            mem : MappedMemory::new(state, cartridge, 0x8000, 0, 32, MemKind::Prg),
        };
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        rom_state.mem.map(0xC000, 16, last, BankKind::Rom);
        Uxrom {
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

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        state.mem.map(0x8000, 16, value as usize, BankKind::Rom);
    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        if system.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(state, addr, value);
        }
    }
}

impl Mapper for Uxrom {
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
    
    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        system.ppu.nametables.write(state, addr, value);
    }
}
