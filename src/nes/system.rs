use std::rc::Rc;
use nes::bus::{AndEqualsAndMask, NotAndMask, Address, DeviceKind, AddressBus};
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};
use nes::cartridge::{Cartridge, CartridgeError};
use nes::{Pages, MemoryBlock};

use std::io::Read;

pub enum Region {
    Ntsc,
    Pal,
}

pub struct Machine {
    pub state: Box<SystemState>,
    pub system: System,
}

#[derive(Default)]
pub struct SystemState {
    pub cpu: CpuState,
    pub ppu: PpuState,
    pub mem: Pages
}

pub struct System {
    region: Region,
    pub ppu: Ppu,
    pub cpu: Cpu,
    pub cartridge: Cartridge,
}

impl Machine {
    pub fn load_rom<T: Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        Cartridge::load(file)
    }

    pub fn new(region: Region, cartridge: Cartridge) -> Machine {
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);
        Machine {
            state: state,
            system: system,
        }
    }

    pub fn tick(&mut self) {
        let mut i = 0;
        loop {
            if i > 30{ return; }
            i += 1;
            self.system.cpu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
        }
    }
}

impl System {
    pub fn new(region: Region, cartridge: Cartridge, state: &mut SystemState) -> System {
        let cpu = Cpu::new(state);
        let ppu = Ppu::new(Region::Ntsc, state);
      
        let mut system = System {
            region: region,
            ppu: ppu,
            cpu: cpu,
            cartridge: cartridge,
        };

        system.cpu.register_read(DeviceKind::Ppu, Address(0x2000));
        system.cpu.register_write(DeviceKind::Ppu, Address(0x2001));
        system.cpu.register_read(DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_write(DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.ppu.register_read(DeviceKind::PpuRam, 
                                 AndEqualsAndMask(0x2800, 0x2000, 0x7ff));
        system.ppu.register_write(DeviceKind::PpuRam,
                                 AndEqualsAndMask(0x2800, 0x2000, 0x7ff));
        system.cartridge.register(&mut system.cpu, &mut system.ppu);

        system
    }
}
