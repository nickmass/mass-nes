use std::rc::Rc;
use nes::bus::{DeviceMappings, RangeAndMask, AndEqualsAndMask, NotAndMask, Address, DeviceKind, AddressBus};
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};
use nes::cartridge::{Cartridge, CartridgeError};
use nes::memory::{Pages, MemoryBlock};
use nes::debug::Debug;

use std::io::Read;

pub enum Region {
    Ntsc,
    Pal,
}
impl Region {
    pub fn default_palette(&self) -> &'static [u8; 192] {
        match *self {
            Region::Ntsc => include_bytes!("default.pal"),
            Region::Pal => include_bytes!("default.pal"),
        }
    }
}


pub struct Machine<FR, FC> where FR: FnMut(&[u8;256*240]), FC: Fn() -> bool {
    pub state: Box<SystemState>,
    pub system: System,
    on_render: FR,
    on_closed: FC,
}

impl<FR, FC> Machine<FR, FC> where FR: FnMut(&[u8;256*240]), FC: Fn() -> bool {

    pub fn new(region: Region, cartridge: Cartridge, render: FR, closed: FC) -> Machine<FR, FC> {
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);
        Machine {
            state: state,
            system: system,
            on_render: render,
            on_closed: closed,
        }
    }

    pub fn run(&mut self) {
        let mut i = 0;
        self.system.cpu.power(&self.system, &mut self.state);
        let mut last_vblank = false;
        loop {
            if i < 0{ return; }
            i += 1;
            self.system.cpu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            if self.state.ppu.vblank && !last_vblank {
                (self.on_render)(&self.state.ppu.screen);
            }
            last_vblank = self.state.ppu.vblank;
            if (self.on_closed)() {
                break;
            }
        }
    }
}

#[derive(Default)]
pub struct SystemState {
    pub cpu: CpuState,
    pub ppu: PpuState,
    pub mem: Pages,
    pub mappings: DeviceMappings,
}

pub struct System {
    region: Region,
    pub ppu: Ppu,
    pub cpu: Cpu,
    pub cartridge: Cartridge,
    pub debug: Debug,
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
            debug: Debug::new(),
        };

        system.cpu.register_read(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_write(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_read(state, DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x7));
        system.cpu.register_write(state, DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x7));
        system.cpu.register_write(state, DeviceKind::Ppu, Address(0x4014));
        system.ppu.register_read(state, DeviceKind::PpuRam, 
                                 AndEqualsAndMask(0x2800, 0x2000, 0x7ff));
        system.ppu.register_write(state, DeviceKind::PpuRam,
                                 AndEqualsAndMask(0x2800, 0x2000, 0x7ff));
        system.cartridge.register(state, &mut system.cpu, &mut system.ppu);

        system
    }
}
