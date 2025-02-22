#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc1 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg: MappedMemory,
    chr: MappedMemory,
    shift_reg: u32,
    counter: u32,
    regs: [u32; 4],
    prg_ram_write_protect: bool,
    last: usize,
    mirroring: SimpleMirroring,
    chr_type: BankKind,
    wide_prg: bool,
}

impl Mmc1 {
    pub fn new(mut cartridge: INes) -> Mmc1 {
        let chr_type = if cartridge.chr_rom.is_empty() {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let chr = match chr_type {
            BankKind::Rom => MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram => MappedMemory::new(&cartridge, 0x0000, 8, 8, MemKind::Chr),
        };

        let mut prg = MappedMemory::new(&cartridge, 0x6000, 8, 40, MemKind::Prg);
        if let Some(wram) = cartridge.wram.take() {
            prg.restore_wram(wram);
        }

        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        let wide_prg = cartridge.prg_rom.len() == 512 * 1024;

        let mut rom = Mmc1 {
            cartridge,
            prg,
            chr,
            shift_reg: 0,
            counter: 0,
            regs: [0x0c, 0, 0, 0],
            prg_ram_write_protect: true,
            last,
            mirroring,
            chr_type,
            wide_prg,
        };

        rom.sync();

        rom
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr & 0x8000 == 0 {
            //prg ram
            if !self.prg_ram_write_protect {
                self.prg.write(addr, value);
            }
            return;
        }

        if value & 0x80 != 0 {
            self.regs[0] |= 0x0c;
            self.sync();
            self.shift_reg = 0;
            self.counter = 0;
        } else {
            self.shift_reg |= ((value as u32 & 1) << self.counter) as u32;
            self.counter += 1;
            if self.counter == 5 {
                match addr & 0xfffe {
                    0x8000 => self.regs[0] = self.shift_reg,
                    0xA000 => self.regs[1] = self.shift_reg,
                    0xC000 => self.regs[2] = self.shift_reg,
                    0xE000 => self.regs[3] = self.shift_reg,
                    _ => unreachable!(),
                }
                self.sync();
                self.shift_reg = 0;
                self.counter = 0;
            }
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        self.chr.write(addr, value);
    }

    fn sync(&mut self) {
        self.prg.map(0x6000, 8, 0, BankKind::Ram);

        match self.regs[0] & 3 {
            0 => self.mirroring.internal_b(),
            1 => self.mirroring.internal_a(),
            2 => self.mirroring.vertical(),
            3 => self.mirroring.horizontal(),
            _ => unreachable!(),
        }

        let prg_high = if self.wide_prg {
            (self.regs[1] & 0x10) as usize
        } else {
            0
        };
        let prg_bank = (self.regs[3] & 0xf) as usize;

        match self.regs[0] & 0xc {
            0 | 0x4 => {
                self.prg
                    .map(0x8000, 32, (prg_bank | prg_high) >> 1, BankKind::Rom);
            }
            0x8 => {
                self.prg.map(0x8000, 16, 0 | prg_high, BankKind::Rom);
                self.prg.map(0xc000, 16, prg_bank | prg_high, BankKind::Rom);
            }
            0xc => {
                self.prg.map(0x8000, 16, prg_bank | prg_high, BankKind::Rom);
                self.prg
                    .map(0xc000, 16, (self.last & 0xf) | prg_high, BankKind::Rom);
            }
            _ => unreachable!(),
        }

        self.prg_ram_write_protect = self.regs[3] & 0x10 != 0;

        let chr_mask = if self.wide_prg { 0x1 } else { 0x1f };

        match self.regs[0] & 0x10 {
            0x0 => {
                self.chr.map(
                    0x0000,
                    8,
                    ((self.regs[1] & chr_mask) >> 1) as usize,
                    self.chr_type,
                );
            }
            0x10 => {
                self.chr
                    .map(0x0000, 4, (self.regs[1] & chr_mask) as usize, self.chr_type);
                self.chr
                    .map(0x1000, 4, (self.regs[2] & chr_mask) as usize, self.chr_type);
            }
            _ => unreachable!(),
        }
    }
}

impl Mapper for Mmc1 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xe001));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
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

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg.save_wram()
        } else {
            None
        }
    }
}
