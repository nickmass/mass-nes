use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};

use std::cell::RefCell;

use super::SimpleMirroring;

pub struct Bf909xState {
    mem: MappedMemory,
}

pub struct Bf909x {
    cartridge: Cartridge,
    chr_ram: MemoryBlock,
    state: RefCell<Bf909xState>,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Bf909x {
    pub fn new(cartridge: Cartridge) -> Bf909x {
        let mut rom_state = Bf909xState {
            mem: MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg),
        };
        let last_prg = (cartridge.prg_rom.len() / 0x4000) - 1;
        rom_state.mem.map(0xC000, 16, last_prg, BankKind::Rom);
        Bf909x {
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

    fn write_cpu(&self, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        match addr & 0xd000 {
            // 0x8000 - 0x9fff is the range for this reg, but it only exists on FireHawk and that game just writes to 0x9000 - 0x9fff
            // this if statement lets us hackily support FireHawk without caring about the submapper
            0x9000 => {
                if value & 0x10 != 0 {
                    self.mirroring.internal_a();
                } else {
                    self.mirroring.internal_b();
                }
            }
            0xc000 | 0xd000 => state
                .mem
                .map(0x8000, 16, (value & 0xf) as usize, BankKind::Rom),
            _ => (),
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        if self.cartridge.chr_ram_bytes > 0 {
            self.chr_ram.write(addr, value);
        }
    }
}

impl Mapper for Bf909x {
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