#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};

use std::cell::RefCell;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct UxromState {
    mem: MappedMemory,
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Uxrom {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: Cartridge,
    chr_ram: MemoryBlock,
    #[cfg_attr(feature = "save-states", save(nested))]
    state: RefCell<UxromState>,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Uxrom {
    pub fn new(cartridge: Cartridge) -> Uxrom {
        let mut rom_state = UxromState {
            mem: MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg),
        };
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        rom_state.mem.map(0xC000, 16, last, BankKind::Rom);
        Uxrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10),
            state: RefCell::new(rom_state),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.state.borrow().mem.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.read(addr)
        } else {
            self.cartridge.chr_rom[addr as usize]
        }
    }

    fn write_cpu(&self, _addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        state.mem.map(0x8000, 16, value as usize, BankKind::Rom);
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(addr, value);
        }
    }
}

impl Mapper for Uxrom {
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
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
