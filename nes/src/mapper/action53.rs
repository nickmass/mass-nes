use crate::bus::{AddressBus, AndAndMask, BusKind, DeviceKind, NotAndMask, RangeAndMask};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::nametables::Nametable;
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

use std::cell::RefCell;

pub struct Action53State {
    prg: MappedMemory,
    chr: MappedMemory,
    regs: [u8; 4],
    reg_index: usize,
}

pub struct Action53 {
    state: RefCell<Action53State>,
}

impl Action53 {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Action53 {
        let chr_type = if cartridge.chr_rom.is_empty() {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let mut chr = match chr_type {
            BankKind::Rom => MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram => MappedMemory::new(state, cartridge, 0x0000, 8, 8, MemKind::Chr),
        };

        let mut prg = MappedMemory::new(state, cartridge, 0x8000, 0, 32, MemKind::Prg);
        let last = (cartridge.prg_rom.len() / 0x4000) - 1;
        prg.map(0x8000, 16, 0, BankKind::Rom);
        prg.map(0xC000, 16, last, BankKind::Rom);
        chr.map(0x0000, 8, 0, BankKind::Ram);

        let regs = [0x00, 0x00, 0x02, 0xff];

        let rom_state = Action53State {
            prg,
            chr,
            regs,
            reg_index: 0,
        };

        Action53 {
            state: RefCell::new(rom_state),
        }
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().prg.read(system, state, addr)
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().chr.read(system, state, addr)
    }

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
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
                self.sync(&mut *rom, &system.ppu, state);
            }
        }
    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        self.state
            .borrow_mut()
            .chr
            .write(system, state, addr, value);
    }

    fn sync(&self, rom: &mut Action53State, ppu: &Ppu, state: &mut SystemState) {
        match rom.reg_index {
            0x00 => {
                if rom.regs[2] & 0x02 == 0 {
                    if rom.regs[0] & 0x10 == 0 {
                        ppu.nametables.set_single(state, Nametable::First);
                    } else {
                        ppu.nametables.set_single(state, Nametable::Second);
                    }
                }
            }
            0x01 => {
                if rom.regs[2] & 0x02 == 0 {
                    if rom.regs[1] & 0x10 == 0 {
                        ppu.nametables.set_single(state, Nametable::First);
                    } else {
                        ppu.nametables.set_single(state, Nametable::Second);
                    }
                }
            }
            0x02 => match rom.regs[2] & 3 {
                0 => ppu.nametables.set_single(state, Nametable::First),
                1 => ppu.nametables.set_single(state, Nametable::Second),
                2 => ppu.nametables.set_vertical(state),
                3 => ppu.nametables.set_horizontal(state),
                _ => unreachable!(),
            },
            0x03 => {}
            _ => unreachable!(),
        }

        let low;
        let high;
        let mode = (rom.regs[2] >> 2) & 0x03;
        let size = (rom.regs[2] >> 4) & 0x03;
        let outer = (rom.regs[3] as usize) << 1;
        let inner = (rom.regs[1] as usize) & 0x0f;
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

        rom.prg.map(0x8000, 16, low as usize, BankKind::Rom);
        rom.prg.map(0xc000, 16, high as usize, BankKind::Rom);
    }
}

impl Mapper for Action53 {
    fn register(
        &self,
        state: &mut SystemState,
        cpu: &mut AddressBus,
        ppu: &mut Ppu,
        _cart: &Cartridge,
    ) {
        cpu.register_write(
            state,
            DeviceKind::Mapper,
            RangeAndMask(0x5000, 0x6000, 0xffff),
        );
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        ppu.register_write(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        let mut rom = self.state.borrow_mut();
        self.sync(&mut *rom, ppu, state);
    }

    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(system, state, addr),
            BusKind::Ppu => self.read_ppu(system, state, addr),
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(system, state, addr),
            BusKind::Ppu => self.read_ppu(system, state, addr),
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(system, state, addr, value),
            BusKind::Ppu => self.write_ppu(system, state, addr, value),
        }
    }
}
