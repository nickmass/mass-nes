use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, NotAndMask};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::nametables::Nametable;
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

use std::cell::RefCell;

pub struct SxromState {
    prg: MappedMemory,
    chr: MappedMemory,
    shift_reg: u32,
    counter: u32,
    regs: [u32; 4],
    prg_ram_write_protect: bool,
    last: usize,
}

pub struct Sxrom {
    state: RefCell<SxromState>,
    chr_type: BankKind,
}

impl Sxrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Sxrom {
        let chr_type = if cartridge.chr_rom.len() == 0 {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let chr = match chr_type {
            BankKind::Rom => MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram => MappedMemory::new(state, cartridge, 0x0000, 8, 8, MemKind::Chr),
        };

        let prg = MappedMemory::new(state, cartridge, 0x6000, 16, 48, MemKind::Prg);

        let rom_state = SxromState {
            prg: prg,
            chr: chr,
            shift_reg: 0,
            counter: 0,
            regs: [0x0c, 0, 0, 0],
            prg_ram_write_protect: true,
            last: (cartridge.prg_rom.len() / 0x4000) - 1,
        };
        let rom = Sxrom {
            state: RefCell::new(rom_state),
            chr_type: chr_type,
        };
        rom
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().prg.read(system, state, addr)
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.state.borrow().chr.read(system, state, addr)
    }

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        if addr & 0x8000 == 0 {
            //prg ram
            if !rom.prg_ram_write_protect {
                rom.prg.write(system, state, addr, value);
            }
            return;
        }
        if value & 0x80 != 0 {
            rom.shift_reg = 0;
            rom.counter = 0;
            self.sync(&mut *rom, &system.ppu, state);
        } else {
            rom.shift_reg |= ((value as u32 & 1) << rom.counter) as u32;
            rom.counter += 1;
            if rom.counter == 5 {
                match addr & 0xfffe {
                    0x8000 => rom.regs[0] = rom.shift_reg,
                    0xA000 => rom.regs[1] = rom.shift_reg,
                    0xC000 => rom.regs[2] = rom.shift_reg,
                    0xE000 => rom.regs[3] = rom.shift_reg,
                    _ => unreachable!(),
                }
                self.sync(&mut *rom, &system.ppu, state);
                rom.shift_reg = 0;
                rom.counter = 0;
            }
        }
    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        self.state
            .borrow_mut()
            .chr
            .write(system, state, addr, value);
    }

    fn sync(&self, rom: &mut SxromState, ppu: &Ppu, state: &mut SystemState) {
        rom.prg.map(0x6000, 16, 0, BankKind::Ram);

        match rom.regs[0] & 3 {
            0 => ppu.nametables.set_single(state, Nametable::First),
            1 => ppu.nametables.set_single(state, Nametable::Second),
            2 => ppu.nametables.set_vertical(state),
            3 => ppu.nametables.set_horizontal(state),
            _ => unreachable!(),
        }
        match rom.regs[0] & 0xc {
            0 | 0x4 => {
                rom.prg
                    .map(0x8000, 32, (rom.regs[3] & 0xf >> 1) as usize, BankKind::Rom);
            }
            0x8 => {
                rom.prg.map(0x8000, 16, 0, BankKind::Rom);
                rom.prg
                    .map(0xc000, 16, (rom.regs[3] & 0xf) as usize, BankKind::Rom);
            }
            0xc => {
                rom.prg
                    .map(0x8000, 16, (rom.regs[3] & 0xf) as usize, BankKind::Rom);
                rom.prg.map(0xc000, 16, rom.last, BankKind::Rom);
            }
            _ => unreachable!(),
        }

        rom.prg_ram_write_protect = rom.regs[3] & 0x10 != 0;

        if rom.counter == 0 {
            rom.prg
                .map(0x8000, 16, (rom.regs[3] & 0xf) as usize, BankKind::Rom);
            rom.prg.map(0x8000, 16, rom.last, BankKind::Rom);
        }

        match rom.regs[0] & 0x10 {
            0x0 => {
                rom.chr
                    .map(0x0000, 8, (rom.regs[1] & 0x1f >> 1) as usize, self.chr_type);
            }
            0x10 => {
                rom.chr
                    .map(0x0000, 4, (rom.regs[1] & 0x1f) as usize, self.chr_type);
                rom.chr
                    .map(0x1000, 4, (rom.regs[2] & 0x1f) as usize, self.chr_type);
            }
            _ => unreachable!(),
        }
    }
}

impl Mapper for Sxrom {
    fn register(
        &self,
        state: &mut SystemState,
        cpu: &mut AddressBus,
        ppu: &mut Ppu,
        cart: &Cartridge,
    ) {
        cpu.register_read(
            state,
            DeviceKind::Mapper,
            AndEqualsAndMask(0xe000, 0x6000, 0x7fff),
        );
        cpu.register_write(
            state,
            DeviceKind::Mapper,
            AndEqualsAndMask(0xe000, 0x6000, 0x7fff),
        );
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xe001));
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

    fn tick(&self, system: &System, state: &mut SystemState) {}

    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        system.ppu.nametables.write(state, addr, value);
    }

    fn update_ppu_addr(&self, system: &System, state: &mut SystemState, addr: u16) {}
}
