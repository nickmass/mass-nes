#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::{Mapper, SimpleMirroring};
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
pub enum NamcoVariant {
    Unspecified,
    Namco175,
    Namco340,
}

impl NamcoVariant {
    fn has_2k_prg_ram(&self) -> bool {
        match self {
            NamcoVariant::Unspecified => false,
            NamcoVariant::Namco175 => true,
            NamcoVariant::Namco340 => false,
        }
    }

    fn has_mirroring_ctrl(&self) -> bool {
        match self {
            NamcoVariant::Unspecified => true,
            NamcoVariant::Namco175 => false,
            NamcoVariant::Namco340 => true,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Namco175_340 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    variant: NamcoVariant,
    prg_ram: Option<MemoryBlock>,
    chr_bank_regs: [u8; 8],
    prg_bank_regs: [u8; 4],
    write_protect: bool,
    mirroring: SimpleMirroring,
}

impl Namco175_340 {
    pub fn new(mut cartridge: INes, variant: NamcoVariant) -> Self {
        let prg_ram = if variant.has_2k_prg_ram() && cartridge.prg_ram_bytes > 0 {
            let mut ram = MemoryBlock::new(2);
            if let Some(wram) = cartridge.wram.take() {
                ram.restore_wram(wram);
            }
            Some(ram)
        } else if cartridge.prg_ram_bytes > 0 {
            let mut ram = MemoryBlock::new(8);
            if let Some(wram) = cartridge.wram.take() {
                ram.restore_wram(wram);
            }
            Some(ram)
        } else {
            None
        };

        let fixed_bank = ((cartridge.prg_rom.len() / 0x2000) - 1) as u8;

        Self {
            variant,
            prg_ram,
            chr_bank_regs: [0; 8],
            prg_bank_regs: [0, 0, 0, fixed_bank],
            write_protect: true,
            mirroring: SimpleMirroring::new(cartridge.mirroring),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7fff => {
                if let Some(ram) = self.prg_ram.as_ref() {
                    if self.variant.has_2k_prg_ram() {
                        if self.write_protect {
                            0
                        } else {
                            ram.read_mapped(0, 2 * 1024, addr)
                        }
                    } else {
                        ram.read_mapped(0, 8 * 1024, addr)
                    }
                } else {
                    0
                }
            }
            0x8000.. => {
                let bank_idx = addr as usize >> 13 & 3;
                let bank = self.prg_bank_regs[bank_idx] as usize;
                self.cartridge.prg_rom.read_mapped(bank, 8 * 1024, addr)
            }
            _ => 0,
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0xbfff => {
                let reg = (addr - 0x8000) / 0x800;
                self.chr_bank_regs[reg as usize] = value;
            }
            0xc000..=0xc7ff if self.variant.has_2k_prg_ram() => {
                self.write_protect = value & 1 == 0;
            }
            0xe000..=0xe7ff => {
                self.prg_bank_regs[0] = value & 0x3f;
                if self.variant.has_mirroring_ctrl() {
                    match value >> 6 {
                        0 => self.mirroring.internal_a(),
                        1 => self.mirroring.vertical(),
                        2 => self.mirroring.internal_b(),
                        3 => self.mirroring.horizontal(),
                        _ => unreachable!(),
                    }
                }
            }
            0xe800..=0xefff => {
                self.prg_bank_regs[1] = value & 0x3f;
            }
            0xf000..=0xf7ff => {
                self.prg_bank_regs[2] = value & 0x3f;
            }
            0x6000..=0x7fff => {
                if let Some(ram) = self.prg_ram.as_mut() {
                    if self.variant.has_2k_prg_ram() {
                        if !self.write_protect {
                            ram.write_mapped(0, 2 * 1024, addr, value);
                        }
                    } else {
                        ram.write_mapped(0, 8 * 1024, addr, value);
                    }
                }
            }
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let bank_idx = (addr as usize >> 10) & 0xf;
        let bank = self.chr_bank_regs[bank_idx] as usize;
        self.cartridge.chr_rom.read_mapped(bank, 1024, addr)
    }
}

impl Mapper for Namco175_340 {
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
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.as_ref().and_then(|r| r.save_wram())
        } else {
            None
        }
    }
}
