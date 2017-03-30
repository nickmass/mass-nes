use system::{System, SystemState};
use memory::{BankKind, MappedMemory, MemKind};
use bus::{DeviceKind, BusKind, AndAndMask, NotAndMask, AndEqualsAndMask};
use cartridge::Cartridge;
use cpu::Cpu;
use ppu::Ppu;
use mapper::Mapper;
use nametables::Nametable;

use std::cell::RefCell;

pub struct Fme7State {
    current_tick: u64,
    prg: MappedMemory,
    chr: MappedMemory,
    command: u8,
    parameter: u8,
    irq_enable: bool,
    irq_counter_enable: bool,
    irq_counter: u16,
    irq: bool,
    ram_protect: bool,
    ram_enable: bool,
}

pub struct Fme7 {
    state: RefCell<Fme7State>,
}

impl Fme7 {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Fme7 {
        let chr = MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr);
        let mut prg = MappedMemory::new(state, cartridge, 0x6000, 16, 48, MemKind::Prg);
        prg.map(0x6000, 16, 0, BankKind::Ram);
        prg.map(0xe000, 8, (cartridge.prg_rom.len() / 0x2000) - 1, BankKind::Rom);
        let rom_state = Fme7State {
            current_tick: 0,
            prg : prg,
            chr : chr,
            command: 0,
            parameter: 0,
            irq_enable: false,
            irq_counter_enable: false,
            irq_counter: 0,
            irq: false,
            ram_protect: false,
            ram_enable: false,
        };
        let rom = Fme7 {
            state: RefCell::new(rom_state),
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
        if addr & 0xe000 == 0x6000 {
            if rom.ram_enable && !rom.ram_protect {
                rom.prg.write(system, state, addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                rom.command = value & 0xf;
            },
            0xa000 => {
                rom.parameter = value;
                self.sync(&mut rom, system, state);
            },
            0xc000 => {
            },
            0xe000 => {
            },
            _ => unreachable!(),
        }

    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        self.state.borrow_mut().chr.write(system, state, addr, value);
    }

    fn sync(&self, rom: &mut Fme7State, system: &System, state: &mut SystemState) {
        match rom.command {
            0...7 => rom.chr.map(0x400 * rom.command as u16, 1, rom.parameter as usize, BankKind::Rom),
            8 => {
                rom.ram_protect = rom.parameter & 0x80 == 0;
                rom.ram_enable = rom.parameter & 0x40 != 0;
                if rom.ram_enable {
                    rom.prg.map(0x6000, 8, 0, BankKind::Ram);
                } else {
                    rom.prg.map(0x6000, 8, (rom.parameter & 0x3f) as usize, BankKind::Rom);
                }
            },
            9 => rom.prg.map(0x8000, 8, (rom.parameter & 0x3f) as usize, BankKind::Rom),
            0xa => rom.prg.map(0xa000, 8, (rom.parameter & 0x3f) as usize, BankKind::Rom),
            0xb => rom.prg.map(0xc000, 8, (rom.parameter & 0x3f) as usize, BankKind::Rom),
            0xc => {
                match rom.parameter & 0x3 {
                    0 => system.ppu.nametables.set_vertical(state),
                    1 => system.ppu.nametables.set_horizontal(state),
                    2 => system.ppu.nametables.set_single(state, Nametable::First),
                    3 => system.ppu.nametables.set_single(state, Nametable::Second),
                    _ => unreachable!(),
                }
            },
            0xd => {
                rom.irq_enable = rom.parameter & 1 != 0;
                rom.irq_counter_enable = rom.parameter & 0x80 != 0;
                rom.irq = false;
            },
            0xe => {
                rom.irq_counter = (rom.irq_counter & 0xff00) | rom.parameter as u16;
            },
            0xf => {
                rom.irq_counter = (rom.irq_counter & 0x00ff) | ((rom.parameter as u16) <<8);
            },
            _ => unreachable!(),
        }
    }
}


impl Mapper for Fme7 {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
    cart: &Cartridge) {
        cpu.register_read(state, DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000,
                                                                    0x7fff));
        cpu.register_write(state, DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000,
                                                                 0x7fff));
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xe000));
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        ppu.register_write(state, DeviceKind::Mapper, NotAndMask(0x1fff));
    }

    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr:u16)
    -> u8 {
        match bus {
            BusKind::Cpu => {
                self.read_cpu(system, state, addr)
            },
            BusKind::Ppu => {
                self.read_ppu(system, state, addr)
            },
        }
    }

    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16)
    -> u8 {
        match bus {
            BusKind::Cpu => {
                self.read_cpu(system, state, addr)
            },
            BusKind::Ppu => {
                self.read_ppu(system, state, addr)
            },
        }
    }

    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState,
    addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => {
                self.write_cpu(system, state, addr, value)
            },
            BusKind::Ppu => {
                self.write_ppu(system, state, addr, value)
            },
        }
    }

    fn tick(&self, system: &System, state: &mut SystemState) {
        let mut rom = self.state.borrow_mut();
        if rom.irq_counter_enable {
            rom.irq_counter = rom.irq_counter.wrapping_sub(1);
            if rom.irq_counter == 0xffff && rom.irq_enable {
                rom.irq = true;
            }
        }
        if rom.irq {
            state.cpu.irq_req();
        }
    }
    
    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        system.ppu.nametables.write(state, addr, value);
    }

    fn update_ppu_addr(&self, system: &System, state: &mut SystemState, addr: u16) {
    }
}
