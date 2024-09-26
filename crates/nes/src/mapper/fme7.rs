#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use std::cell::RefCell;

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Fme7State {
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
    mirroring: SimpleMirroring,
}

impl Fme7State {
    fn sync(&mut self) {
        match self.command {
            0..=7 => self.chr.map(
                0x400 * self.command as u16,
                1,
                self.parameter as usize,
                BankKind::Rom,
            ),
            8 => {
                self.ram_protect = self.parameter & 0x80 == 0;
                self.ram_enable = self.parameter & 0x40 != 0;
                if self.ram_enable {
                    self.prg.map(0x6000, 8, 0, BankKind::Ram);
                } else {
                    self.prg
                        .map(0x6000, 8, (self.parameter & 0x3f) as usize, BankKind::Rom);
                }
            }
            9 => self
                .prg
                .map(0x8000, 8, (self.parameter & 0x3f) as usize, BankKind::Rom),
            0xa => self
                .prg
                .map(0xa000, 8, (self.parameter & 0x3f) as usize, BankKind::Rom),
            0xb => self
                .prg
                .map(0xc000, 8, (self.parameter & 0x3f) as usize, BankKind::Rom),
            0xc => match self.parameter & 0x3 {
                0 => self.mirroring.vertical(),
                1 => self.mirroring.horizontal(),
                2 => self.mirroring.internal_a(),
                3 => self.mirroring.internal_b(),
                _ => unreachable!(),
            },
            0xd => {
                self.irq_enable = self.parameter & 1 != 0;
                self.irq_counter_enable = self.parameter & 0x80 != 0;
                self.irq = false;
            }
            0xe => {
                self.irq_counter = (self.irq_counter & 0xff00) | self.parameter as u16;
            }
            0xf => {
                self.irq_counter = (self.irq_counter & 0x00ff) | ((self.parameter as u16) << 8);
            }
            _ => unreachable!(),
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Fme7 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: Cartridge,
    #[cfg_attr(feature = "save-states", save(nested))]
    state: RefCell<Fme7State>,
}

impl Fme7 {
    pub fn new(cartridge: Cartridge) -> Fme7 {
        let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
        let mut prg = MappedMemory::new(&cartridge, 0x6000, 16, 48, MemKind::Prg);
        prg.map(0x6000, 16, 0, BankKind::Ram);
        prg.map(
            0xe000,
            8,
            (cartridge.prg_rom.len() / 0x2000) - 1,
            BankKind::Rom,
        );

        let mut rom_state = Fme7State {
            prg,
            chr,
            command: 0,
            parameter: 0,
            irq_enable: false,
            irq_counter_enable: false,
            irq_counter: 0,
            irq: false,
            ram_protect: false,
            ram_enable: false,
            mirroring: SimpleMirroring::new(cartridge.mirroring.into()),
        };

        rom_state.sync();

        Fme7 {
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
        if addr & 0xe000 == 0x6000 {
            if rom.ram_enable && !rom.ram_protect {
                rom.prg.write(addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                rom.command = value & 0xf;
            }
            0xa000 => {
                rom.parameter = value;
                rom.sync();
            }
            0xc000 => {}
            0xe000 => {}
            _ => unreachable!(),
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        self.state.borrow_mut().chr.write(addr, value);
    }
}

impl Mapper for Fme7 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xe000));
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

    fn tick(&self) {
        let mut rom = self.state.borrow_mut();
        if rom.irq_counter_enable {
            rom.irq_counter = rom.irq_counter.wrapping_sub(1);
            if rom.irq_counter == 0xffff && rom.irq_enable {
                rom.irq = true;
            }
        }
    }

    fn get_irq(&self) -> bool {
        let rom = self.state.borrow();
        rom.irq
    }

    fn ppu_fetch(&self, address: u16) -> super::Nametable {
        let rom = self.state.borrow();
        rom.mirroring.ppu_fetch(address)
    }
}
