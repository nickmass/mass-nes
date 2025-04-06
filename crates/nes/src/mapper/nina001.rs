#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nina001 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg_ram: FixedMemoryBlock<8>,
    prg_bank: u8,
    chr_banks: [u8; 2],
}

impl Nina001 {
    pub fn new(mut cartridge: INes) -> Nina001 {
        let mut prg_ram = FixedMemoryBlock::new();
        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }

        Self {
            cartridge,
            prg_ram,
            prg_bank: 0,
            chr_banks: [0; 2],
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0x8000 == 0 {
            self.prg_ram.read(addr)
        } else {
            self.cartridge
                .prg_rom
                .read_mapped(self.prg_bank as usize, 32 * 1024, addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x7ffd => self.prg_bank = value & 0x3,
            0x7ffe => self.chr_banks[0] = value & 0xf,
            0x7fff => self.chr_banks[1] = value & 0xf,
            _ => (),
        }

        self.prg_ram.write(addr, value);
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let bank_idx = if addr & 0x1000 == 0 { 0 } else { 1 };
        let bank = self.chr_banks[bank_idx] as usize;
        self.cartridge.chr_rom.read_mapped(bank, 4 * 1024, addr)
    }
}

impl Mapper for Nina001 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.cartridge.mirroring.ppu_fetch(address)
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.save_wram()
        } else {
            None
        }
    }
}
