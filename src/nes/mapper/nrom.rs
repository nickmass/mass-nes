use system::{System, SystemState};
use memory::MemoryBlock;
use bus::{DeviceKind, BusKind, AndAndMask, NotAndMask};
use cartridge::{Mirroring, Cartridge};
use cpu::Cpu;
use ppu::Ppu;
use mapper::Mapper;

pub struct Nrom {
    chr_ram: MemoryBlock,
    prg_ram: MemoryBlock,
}

impl Nrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Nrom {
        Nrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10, &mut state.mem),
            prg_ram: MemoryBlock::new(cartridge.prg_ram_bytes >> 10, &mut state.mem),
        }
    }
}

impl Mapper for Nrom{
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
    cart: &Cartridge) {
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000,
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
                system.cartridge.prg_rom[addr as usize]
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.peek(state, addr)
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
                system.cartridge.prg_rom[addr as usize]
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(state, addr)
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
            },
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.write(state, addr, value);
                }
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

    fn update_ppu_addr(&self, system: &System, state: &mut SystemState, addr: u16) {}
}
