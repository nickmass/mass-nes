use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};

use std::cell::RefCell;

use super::SimpleMirroring;

#[derive(SaveState)]
pub struct AxromState {
    prg: MappedMemory,
}

#[derive(SaveState)]
pub struct Axrom {
    #[save(skip)]
    cartridge: Cartridge,
    chr_ram: MemoryBlock,
    #[save(nested)]
    state: RefCell<AxromState>,
    mirroring: SimpleMirroring,
}

impl Axrom {
    pub fn new(cartridge: Cartridge) -> Axrom {
        let mut rom_state = AxromState {
            prg: MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg),
        };
        rom_state.prg.map(0x8000, 32, 0, BankKind::Rom);
        Axrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10),
            state: RefCell::new(rom_state),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.state.borrow().prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.read(addr)
        } else {
            self.cartridge.chr_rom[addr as usize]
        }
    }

    fn write_cpu(&self, _addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        rom.prg.map(0x8000, 32, (value & 7) as usize, BankKind::Rom);
        if value & 0x10 == 0 {
            self.mirroring.internal_a()
        } else {
            self.mirroring.internal_b()
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(addr, value);
        }
    }
}

impl Mapper for Axrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
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

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
