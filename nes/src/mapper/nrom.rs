use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, NotAndMask};
use crate::cartridge::{Cartridge, Mirroring};
use crate::mapper::Mapper;
use crate::memory::MemoryBlock;
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

pub struct Nrom {
    chr_ram: MemoryBlock,
}

impl Nrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Nrom {
        Nrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10, &mut state.mem),
        }
    }
}

impl Mapper for Nrom {
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
            BusKind::Cpu => system.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.peek(state, addr)
                } else {
                    system.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => system.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(state, addr)
                } else {
                    system.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {}
            BusKind::Ppu => {
                if system.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.write(state, addr, value);
                }
            }
        }
    }
}
