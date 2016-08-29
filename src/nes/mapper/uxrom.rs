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
                self.state.borrow().mem.read(system, state, addr)
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.peek(bus, state, addr)
                } else {
                    system.cartridge.chr_rom[addr as usize]
                }
            },
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16)
    -> u8 {
        match bus {
            BusKind::Cpu => {
                self.state.borrow().mem.read(system, state, addr)
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(bus, state, addr)
                } else {
                    system.cartridge.chr_rom[addr as usize]
                }
            },
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState,
    addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                    let mut state = self.state.borrow_mut();
                    state.mem.map(0x8000, 16, value as usize, BankKind::Rom);
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.write(bus, state, addr, value);
                }
            },
        }
    }
}
