use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::{Mapper, SimpleMirroring};
use crate::memory::MemoryBlock;

pub struct Nrom {
    cartridge: Cartridge,
    chr_ram: MemoryBlock,
    mirroring: SimpleMirroring,
    prg_len: usize,
}

impl Nrom {
    pub fn new(cartridge: Cartridge) -> Nrom {
        Nrom {
            chr_ram: MemoryBlock::new(cartridge.chr_ram_bytes >> 10),
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            prg_len: cartridge.prg_rom.len(),
            cartridge,
        }
    }
}

impl Mapper for Nrom {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(
            DeviceKind::Mapper,
            AndAndMask(0x8000, (self.prg_len - 1) as u16),
        );
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(addr)
                } else {
                    self.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn read(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.cartridge.prg_rom[addr as usize],
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.read(addr)
                } else {
                    self.cartridge.chr_rom[addr as usize]
                }
            }
        }
    }

    fn write(&self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => (),
            BusKind::Ppu => {
                if self.cartridge.chr_ram_bytes > 0 {
                    self.chr_ram.write(addr, value);
                }
            }
        }
    }

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }
}