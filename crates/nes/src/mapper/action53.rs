#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Action53 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    chr_ram: Option<MemoryBlock>,
    regs: [u8; 4],
    mirroring: SimpleMirroring,
    reg_index: usize,
}

impl Action53 {
    pub fn new(cartridge: INes) -> Action53 {
        let chr_ram = cartridge
            .chr_rom
            .is_empty()
            .then(|| MemoryBlock::new(cartridge.chr_ram_bytes / 1024));

        let regs = [0x00, 0x00, 0x02, 0xff];
        let mirroring = SimpleMirroring::new(cartridge.mirroring);

        Self {
            cartridge,
            chr_ram,
            regs,
            reg_index: 0,
            mirroring,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        let bank = self.map_prg(addr);
        self.cartridge.prg_rom.read_mapped(bank, 16 * 1024, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr & 0xf000 {
            0x5000 => match value & 0x81 {
                0x00 => self.reg_index = 0,
                0x01 => self.reg_index = 1,
                0x80 => self.reg_index = 2,
                0x81 => self.reg_index = 3,
                _ => unreachable!(),
            },
            _ => {
                let index = self.reg_index & 3;
                self.regs[index] = value;
                match index {
                    0x00 => {
                        if self.regs[2] & 0x02 == 0 {
                            if self.regs[0] & 0x10 == 0 {
                                self.mirroring.internal_b();
                            } else {
                                self.mirroring.internal_a();
                            }
                        }
                    }
                    0x01 => {
                        if self.regs[2] & 0x02 == 0 {
                            if self.regs[1] & 0x10 == 0 {
                                self.mirroring.internal_b();
                            } else {
                                self.mirroring.internal_a();
                            }
                        }
                    }
                    0x02 => match self.regs[2] & 3 {
                        0 => self.mirroring.internal_b(),
                        1 => self.mirroring.internal_a(),
                        2 => self.mirroring.vertical(),
                        3 => self.mirroring.horizontal(),
                        _ => unreachable!(),
                    },
                    _ => (),
                }
            }
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let bank = self.regs[0] as usize & 3;
        if let Some(ram) = self.chr_ram.as_ref() {
            ram.read_mapped(bank, 8 * 1024, addr)
        } else {
            self.cartridge.chr_rom.read_mapped(bank, 8 * 1024, addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if let Some(ram) = self.chr_ram.as_mut() {
            let bank = self.regs[0] as usize & 3;
            ram.write_mapped(bank, 8 * 1024, addr, value);
        }
    }

    fn map_prg(&self, addr: u16) -> usize {
        let low;
        let high;
        let mode = (self.regs[2] >> 2) & 0x03;
        let size = (self.regs[2] >> 4) & 0x03;
        let outer = (self.regs[3] as usize) << 1;
        let inner = (self.regs[1] as usize) & 0x0f;
        match mode {
            0x00 | 0x01 => match size {
                0x00 => {
                    low = outer;
                    high = outer | 0x01;
                }
                0x01 => {
                    low = (outer & 0xffc) | ((inner & 0x1) << 1);
                    high = (outer & 0xffc) | ((inner & 0x1) << 1) | 0x01;
                }
                0x02 => {
                    low = (outer & 0xff8) | ((inner & 0x3) << 1);
                    high = (outer & 0xff8) | ((inner & 0x3) << 1) | 0x01;
                }
                0x03 => {
                    low = (outer & 0xff0) | ((inner & 0x7) << 1);
                    high = (outer & 0xff0) | ((inner & 0x7) << 1) | 0x01;
                }
                _ => unreachable!(),
            },
            0x02 => {
                low = outer;
                match size {
                    0x00 => high = (outer & 0xffe) | (inner & 0x1),
                    0x01 => high = (outer & 0xffc) | (inner & 0x3),
                    0x02 => high = (outer & 0xff8) | (inner & 0x7),
                    0x03 => high = (outer & 0xff0) | (inner & 0xf),
                    _ => unreachable!(),
                }
            }
            0x03 => {
                high = outer | 0x01;
                match size {
                    0x00 => low = (outer & 0xffe) | (inner & 0x1),
                    0x01 => low = (outer & 0xffc) | (inner & 0x3),
                    0x02 => low = (outer & 0xff8) | (inner & 0x7),
                    0x03 => low = (outer & 0xff0) | (inner & 0xf),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        if addr & 0x4000 == 0 { low } else { high }
    }
}

impl Mapper for Action53 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_write(DeviceKind::Mapper, RangeAndMask(0x5000, 0x6000, 0xffff));
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
}
