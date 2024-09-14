use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use std::cell::RefCell;

use super::SimpleMirroring;

pub struct Action53State {
    prg: MappedMemory,
    chr: MappedMemory,
    regs: [u8; 4],
    mirroring: SimpleMirroring,
    reg_index: usize,
}

impl Action53State {
    fn sync(&mut self) {
        match self.reg_index {
            0x00 => {
                if self.regs[2] & 0x02 == 0 {
                    if self.regs[0] & 0x10 == 0 {
                        self.mirroring.internal_a();
                    } else {
                        self.mirroring.internal_b();
                    }
                }
            }
            0x01 => {
                if self.regs[2] & 0x02 == 0 {
                    if self.regs[1] & 0x10 == 0 {
                        self.mirroring.internal_a();
                    } else {
                        self.mirroring.internal_b();
                    }
                }
            }
            0x02 => match self.regs[2] & 3 {
                0 => self.mirroring.internal_a(),
                1 => self.mirroring.internal_b(),
                2 => self.mirroring.vertical(),
                3 => self.mirroring.horizontal(),
                _ => unreachable!(),
            },
            0x03 => {}
            _ => unreachable!(),
        }

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
        }

        self.prg.map(0x8000, 16, low as usize, BankKind::Rom);
        self.prg.map(0xc000, 16, high as usize, BankKind::Rom);
    }
}

pub struct Action53 {
    cartridge: Cartridge,
    state: RefCell<Action53State>,
}

impl Action53 {
    pub fn new(cartridge: Cartridge) -> Action53 {
        let chr_type = if cartridge.chr_rom.is_empty() {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let mut chr = match chr_type {
            BankKind::Rom => MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram => MappedMemory::new(&cartridge, 0x0000, 8, 8, MemKind::Chr),
        };

        let mut prg = MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg);
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        prg.map(0x8000, 16, 0, BankKind::Rom);
        prg.map(0xC000, 16, last, BankKind::Rom);
        chr.map(0x0000, 8, 0, BankKind::Ram);

        let regs = [0x00, 0x00, 0x02, 0xff];

        let mut rom_state = Action53State {
            prg,
            chr,
            regs,
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
            reg_index: 0,
        };

        rom_state.sync();

        Action53 {
            state: RefCell::new(rom_state),
            cartridge,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.state.borrow().prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.state.borrow().chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&self, addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        match addr & 0xf000 {
            0x5000 => match value & 0x81 {
                0x00 => rom.reg_index = 0,
                0x01 => rom.reg_index = 1,
                0x80 => rom.reg_index = 2,
                0x81 => rom.reg_index = 3,
                _ => unreachable!(),
            },
            _ => {
                let index = rom.reg_index & 3;
                rom.regs[index] = value;
                rom.sync();
            }
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        self.state.borrow_mut().chr.write(addr, value);
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
        let rom = self.state.borrow();
        rom.mirroring.ppu_fetch(address)
    }
}