use nes::system::{System, SystemState};
use nes::memory::{BankKind, MappedMemory, MemKind};
use nes::bus::{DeviceKind, BusKind, AndAndMask, NotAndMask, AndEqualsAndMask};
use nes::cartridge::Cartridge;
use nes::cpu::Cpu;
use nes::ppu::Ppu;
use nes::mapper::Mapper;

use std::cell::RefCell;

pub struct TxromState {
    prg: MappedMemory,
    chr: MappedMemory,
    bank_data: [u8;8],
    bank_select: u8,
    ram_protect: bool,
    ram_enabled: bool,
    irq: bool,
    irq_enabled: bool,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload_pending: bool,
    was_a12_low: bool,
    last: usize,
}

pub struct Txrom {
    state: RefCell<TxromState>,
    chr_type: BankKind,
}

impl Txrom {
    pub fn new(cartridge: &Cartridge, state: &mut SystemState) -> Txrom {
        let chr_type = if cartridge.chr_rom.len() == 0 {
            BankKind::Ram
        } else {
            BankKind::Rom
        };
        let chr = match chr_type {
            BankKind::Rom => 
                MappedMemory::new(state, cartridge, 0x0000, 0, 8, MemKind::Chr),
            BankKind::Ram =>
                MappedMemory::new(state, cartridge, 0x0000, 8, 8, MemKind::Chr),
        };
        
        let mut prg = MappedMemory::new(state, cartridge, 0x6000, 16, 48, MemKind::Prg);
        prg.map(0x6000, 16, 0, BankKind::Ram);

        let rom_state = TxromState {
            prg : prg,
            chr : chr,
            bank_data: [0;8],
            bank_select: 0,
            ram_protect: false,
            ram_enabled: false,
            irq: false,
            irq_enabled: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload_pending: false,
            was_a12_low: false,
            last: (cartridge.prg_rom.len() / 0x2000) -1 
        };
        let rom = Txrom {
            state: RefCell::new(rom_state),
            chr_type: chr_type,
        };
        rom
    }

    fn read_cpu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        let rom = self.state.borrow();
        if addr & 0xe000 == 0x6000 && !rom.ram_enabled {
            (addr & 0xff) as u8
        } else {
            self.state.borrow().prg.read(system, state, addr)
        }
    }

    fn read_ppu(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        self.irq_tick(addr);
        self.state.borrow().chr.read(system, state, addr)
    }

    fn write_cpu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut rom = self.state.borrow_mut();
        if addr & 0xe000 == 0x6000 {
            if rom.ram_enabled && !rom.ram_protect {
                rom.prg.write(system, state, addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                rom.bank_select = value;
                self.sync(&mut rom);
            },
            0x8001 => {
                rom.bank_data[(rom.bank_select & 0x7) as usize] = value;
                self.sync(&mut rom);
            },
            0xa000 => {
                match value & 1 {
                    0 => system.ppu.nametables.set_vertical(state),
                    1 => system.ppu.nametables.set_horizontal(state),
                    _ => unreachable!(),
                }
            },
            0xa001 => {
                rom.ram_protect = value & 0x40 != 0;
                rom.ram_enabled = value & 0x80 != 0;
            },
            0xc000 => rom.irq_latch = value,
            0xc001 => rom.irq_reload_pending = true,
            0xe000 => {
                rom.irq = false;
                rom.irq_enabled = false;
            },
            0xe001 => {
                rom.irq_enabled = true;
            },
            _ => unreachable!(),
        }

    }

    fn write_ppu(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        self.irq_tick(addr);
        self.state.borrow_mut().chr.write(system, state, addr, value);
    }

    fn irq_tick(&self, addr: u16) {
        let mut rom = self.state.borrow_mut();
        let a12 = addr >> 12 & 1 == 1;
        if rom.was_a12_low && a12 {
            if rom.irq_counter == 0 || rom.irq_reload_pending {
                rom.irq_counter = rom.irq_latch;
                rom.irq_reload_pending = false;
            } else {
                rom.irq_counter -= 1;
                if rom.irq_counter == 0 && rom.irq_enabled {
                    rom.irq = true;
                }
            }
        }
        rom.was_a12_low = !a12;
    }

    fn sync(&self, rom: &mut TxromState) {
        if rom.bank_select & 0x80 == 0{
            rom.chr.map(0x0000, 1, (rom.bank_data[0] & 0xfe) as usize, BankKind::Rom); 
            rom.chr.map(0x0400, 1, (rom.bank_data[0] | 0x1) as usize, BankKind::Rom); 
            rom.chr.map(0x0800, 1, (rom.bank_data[1] & 0xfe) as usize, BankKind::Rom); 
            rom.chr.map(0x0c00, 1, (rom.bank_data[1] | 0x01) as usize, BankKind::Rom); 
            rom.chr.map(0x1000, 1, rom.bank_data[2] as usize, BankKind::Rom); 
            rom.chr.map(0x1400, 1, rom.bank_data[3] as usize, BankKind::Rom); 
            rom.chr.map(0x1800, 1, rom.bank_data[4] as usize, BankKind::Rom); 
            rom.chr.map(0x1c00, 1, rom.bank_data[5] as usize, BankKind::Rom); 
        } else {
            rom.chr.map(0x0000, 1, rom.bank_data[2] as usize, BankKind::Rom); 
            rom.chr.map(0x0400, 1, rom.bank_data[3] as usize, BankKind::Rom); 
            rom.chr.map(0x0800, 1, rom.bank_data[4] as usize, BankKind::Rom); 
            rom.chr.map(0x0c00, 1, rom.bank_data[5] as usize, BankKind::Rom); 
            rom.chr.map(0x1000, 1, (rom.bank_data[0] & 0xfe) as usize, BankKind::Rom); 
            rom.chr.map(0x1400, 1, (rom.bank_data[0] | 0x1) as usize, BankKind::Rom); 
            rom.chr.map(0x1800, 1, (rom.bank_data[1] & 0xfe) as usize, BankKind::Rom); 
            rom.chr.map(0x1c00, 1, (rom.bank_data[1] | 0x01) as usize, BankKind::Rom); 
        }

        if rom.bank_select & 0x40 == 0{ 
            rom.prg.map(0x8000, 8, rom.bank_data[6] as usize, BankKind::Rom); 
            rom.prg.map(0xa000, 8, rom.bank_data[7] as usize, BankKind::Rom); 
            rom.prg.map(0xc000, 8, (rom.last - 1) as usize, BankKind::Rom); 
            rom.prg.map(0xe000, 8, rom.last as usize, BankKind::Rom); 
        } else {
            rom.prg.map(0x8000, 8, (rom.last - 1) as usize, BankKind::Rom); 
            rom.prg.map(0xa000, 8, rom.bank_data[7] as usize, BankKind::Rom); 
            rom.prg.map(0xc000, 8, rom.bank_data[6] as usize, BankKind::Rom); 
            rom.prg.map(0xe000, 8, rom.last as usize, BankKind::Rom); 
        }
    }
}


impl Mapper for Txrom {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
    cart: &Cartridge) {
        cpu.register_read(state, DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000,
                                                                    0x7fff));
        cpu.register_write(state, DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000,
                                                                 0x7fff));
        cpu.register_read(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(state, DeviceKind::Mapper, AndAndMask(0x8000, 0xe001));
        ppu.register_read(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        ppu.register_write(state, DeviceKind::Mapper, NotAndMask(0x1fff));
        let mut rom = self.state.borrow_mut();
        self.sync(&mut rom);
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
        if self.state.borrow().irq {
            state.cpu.irq_req();
        }
    }
}
