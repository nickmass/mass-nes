use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use std::cell::RefCell;

use super::SimpleMirroring;

pub struct CnromState {
    chr: MappedMemory,
}

pub struct Cnrom {
    cartridge: Cartridge,
    state: RefCell<CnromState>,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Cnrom {
    pub fn new(cartridge: Cartridge) -> Cnrom {
        let rom_state = CnromState {
            chr: MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr),
        };
        Cnrom {
            state: RefCell::new(rom_state),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.cartridge.prg_rom[addr as usize]
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.state.borrow().chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&self, _addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        state.chr.map(0x0000, 8, (value) as usize, BankKind::Rom);
    }
}

impl Mapper for Cnrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(
            DeviceKind::Mapper,
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
        cpu.register_write(
            DeviceKind::Mapper,
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
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
            BusKind::Ppu => (),
        }
    }

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
