#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc1 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    current_tick: u64,
    last_write_tick: u64,
    prg_ram: FixedMemoryBlock<8>,
    chr_ram: Option<FixedMemoryBlock<8>>,
    shift_reg: u32,
    counter: u32,
    regs: [u32; 4],
    prg_ram_write_protect: bool,
    last: usize,
    mirroring: SimpleMirroring,
    wide_prg: bool,
}

impl Mmc1 {
    pub fn new(mut cartridge: INes) -> Mmc1 {
        let mut prg_ram = FixedMemoryBlock::new();
        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }

        let chr_ram = cartridge
            .chr_rom
            .is_empty()
            .then(|| FixedMemoryBlock::new());

        let mirroring = SimpleMirroring::new(cartridge.mirroring);
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        let wide_prg = cartridge.prg_rom.len() == 512 * 1024;

        Self {
            cartridge,
            current_tick: 0,
            last_write_tick: 0,
            prg_ram,
            chr_ram,
            shift_reg: 0,
            counter: 0,
            regs: [0x0c, 0, 0, 0],
            prg_ram_write_protect: true,
            last,
            mirroring,
            wide_prg,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr & 0x8000 == 0 {
            self.prg_ram.read(addr)
        } else {
            let (bank, size) = self.map_prg(addr);
            self.cartridge.prg_rom.read_mapped(bank, size, addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr & 0x8000 == 0 {
            //prg ram
            if !self.prg_ram_write_protect {
                self.prg_ram.write(addr, value);
            }
            return;
        }

        if value & 0x80 != 0 {
            self.regs[0] |= 0x0c;
            self.shift_reg = 0;
            self.counter = 0;
        } else {
            if self.current_tick > self.last_write_tick + 1 {
                self.shift_reg |= ((value as u32 & 1) << self.counter) as u32;
                self.counter += 1;
                if self.counter == 5 {
                    match addr & 0xe000 {
                        0x8000 => {
                            self.regs[0] = self.shift_reg;
                            match self.regs[0] & 3 {
                                0 => self.mirroring.internal_b(),
                                1 => self.mirroring.internal_a(),
                                2 => self.mirroring.vertical(),
                                3 => self.mirroring.horizontal(),
                                _ => unreachable!(),
                            }
                        }
                        0xA000 => self.regs[1] = self.shift_reg,
                        0xC000 => self.regs[2] = self.shift_reg,
                        0xE000 => {
                            self.regs[3] = self.shift_reg;
                            self.prg_ram_write_protect = self.regs[3] & 0x10 != 0;
                        }
                        _ => unreachable!(),
                    }
                    self.shift_reg = 0;
                    self.counter = 0;
                }
            }
        }
        self.last_write_tick = self.current_tick;
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let (bank, size) = self.map_chr(addr);
        if let Some(ram) = self.chr_ram.as_ref() {
            ram.read_mapped(bank, size, addr)
        } else {
            self.cartridge.chr_rom.read_mapped(bank, size, addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        let (bank, size) = self.map_chr(addr);
        if let Some(ram) = self.chr_ram.as_mut() {
            ram.write_mapped(bank, size, addr, value);
        }
    }

    fn map_prg(&self, addr: u16) -> (usize, usize) {
        let prg_high = if self.wide_prg {
            (self.regs[1] & 0x10) as usize
        } else {
            0
        };
        let prg_bank = (self.regs[3] & 0xf) as usize | prg_high;

        let (bank, size) = match self.regs[0] & 0xc {
            0 | 0x4 => (prg_bank >> 1, 32),
            0x8 if addr & 0x4000 == 0 => (prg_high, 16),
            0x8 => (prg_bank, 16),
            0xc if addr & 0x4000 == 0 => (prg_bank, 16),
            0xc => (self.last & 0xf | prg_high, 16),
            _ => unreachable!(),
        };

        (bank, size * 1024)
    }

    fn map_chr(&self, addr: u16) -> (usize, usize) {
        let chr_mask = if self.wide_prg { 0x1 } else { 0x1f };
        let (bank, size) = match self.regs[0] & 0x10 {
            0x0 => (((self.regs[1] & chr_mask) >> 1) as usize, 8),
            0x10 if addr & 0x1000 == 0 => ((self.regs[1] & chr_mask) as usize, 4),
            0x10 => ((self.regs[2] & chr_mask) as usize, 4),
            _ => unreachable!(),
        };
        (bank, size * 1024)
    }
}

impl Mapper for Mmc1 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
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
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn tick(&mut self) {
        self.current_tick += 1;
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.save_wram()
        } else {
            None
        }
    }
}
