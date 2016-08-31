use nes::bus::{DeviceMappings, RangeAndMask, NotAndMask, Address, DeviceKind};
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};
use nes::cartridge::Cartridge;
use nes::memory::Pages;
use nes::debug::{Debug, DebugState};
use nes::input::{Input, InputState};

pub use nes::input::{Controller, InputDevice};

pub enum Region {
    Ntsc,
    Pal,
}
impl Region {
    pub fn default_palette(&self) -> &'static [u8; 1536] {
        match *self {
            Region::Ntsc => include_bytes!("default.pal"),
            Region::Pal => include_bytes!("default.pal"),
        }
    }
}


pub struct Machine<FR, FC, FI, I, FD> where 
    FR: FnMut(&[u16;256*240]), 
    FC: FnMut() -> bool, 
    FI: FnMut() -> I,
    FD: FnMut(&System, &mut SystemState),
    I: InputDevice {
    
    pub state: Box<SystemState>,
    pub system: System,
    on_render: FR,
    on_closed: FC,
    on_input: FI,
    on_debug: FD,
}

impl<FR, FC, FI, I, FD> Machine<FR, FC, FI, I, FD> where 
    FR: FnMut(&[u16;256*240]),
    FC: FnMut() -> bool,
    FI: FnMut() -> I,
    FD: FnMut(&System, &mut SystemState),
    I: InputDevice {

    pub fn new(region: Region, cartridge: Cartridge, render: FR, closed: FC, input: FI,
               debug: FD) -> Machine<FR, FC, FI, I, FD> {
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);
        Machine {
            state: state,
            system: system,
            on_render: render,
            on_closed: closed,
            on_input: input,
            on_debug: debug,
        }
    }

    pub fn run(&mut self) {
        self.system.cpu.power(&self.system, &mut self.state);
        let mut last_vblank = false;
        loop {
            self.system.cpu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            if self.state.ppu.in_vblank && !last_vblank {
                (self.on_render)(&self.state.ppu.screen);
                let input = (self.on_input)().to_byte();
                self.state.input.input = input;
                (self.on_debug)(&self.system, &mut self.state);
            }
            last_vblank = self.state.ppu.in_vblank;
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
    pub input: InputState,
    pub debug: DebugState,
}

pub struct System {
    region: Region,
    pub ppu: Ppu,
    pub cpu: Cpu,
    pub cartridge: Cartridge,
    pub debug: Debug,
    pub input: Input,
}

impl System {
    pub fn new(region: Region, mut cartridge: Cartridge,
               state: &mut SystemState) -> System {
        let cpu = Cpu::new(state);
        let ppu = Ppu::new(Region::Ntsc, state);
        cartridge.init(state, &cpu, &ppu);

        let mut system = System {
            region: region,
            ppu: ppu,
            cpu: cpu,
            cartridge: cartridge,
            debug: Debug::new(),
            input: Input::new(),
        };

        system.cpu.register_read(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_write(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_read(state, DeviceKind::Ppu,
                                 RangeAndMask(0x2000, 0x4000, 0x2007));
        system.cpu.register_write(state, DeviceKind::Ppu,
                                  RangeAndMask(0x2000, 0x4000, 0x2007));
        system.cpu.register_write(state, DeviceKind::Ppu, Address(0x4014));
        system.ppu.register_read(state, DeviceKind::Nametables, 
                                 RangeAndMask(0x2000, 0x4000, 0xfff));
        system.ppu.register_write(state, DeviceKind::Nametables,
                                 RangeAndMask(0x2000, 0x4000, 0xfff));
        system.cpu.register_read(state, DeviceKind::Input, Address(0x4016));
        system.cpu.register_read(state, DeviceKind::Input, Address(0x4017));
        system.cpu.register_write(state, DeviceKind::Input, Address(0x4016));
        system.cartridge.mapper.register(state, &mut system.cpu, &mut system.ppu,
                               &system.cartridge);

        system
    }
}
