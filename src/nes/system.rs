use std::rc::Rc;
use nes::bus::AddressBus;
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};
use nes::cartridge::{Cartridge, CartridgeError};

use std::io::Read;

pub enum Region {
    Ntsc,
    Pal,
}

#[derive(Default)]
pub struct SystemState {
    pub cpu: CpuState,
    pub ppu: PpuState,
}

pub struct System {
    region: Region,
    cpu_bus: Rc<AddressBus>,
    ppu_bus: Rc<AddressBus>,
    ppu: Rc<Ppu>,
    cpu: Rc<Cpu>,
    state: Box<SystemState>,
    cartridge: Rc<Cartridge>,
}


impl System {
    pub fn load_rom<T: Read>(file: &mut T) -> Result<Cartridge, CartridgeError> {
        Cartridge::load(file)
    }

    pub fn new(region: Region, cartridge: Cartridge) -> System {
        let mut cpu_bus = AddressBus::new();
        let mut ppu_bus = AddressBus::new();
        
        let mut cpu = Cpu::new();
        let mut ppu = Ppu::new(Region::Ntsc ,&mut cpu_bus,&mut ppu_bus);
       
        let rc_cpu_bus = Rc::new(cpu_bus);
        let rc_ppu_bus = Rc::new(ppu_bus);
        let rc_cartridge = Rc::new(cartridge);

        cpu.init(rc_cpu_bus.clone());
        ppu.init(rc_cpu_bus.clone(), rc_ppu_bus.clone());
        
        let system = System {
            region: region,
            cpu_bus: rc_cpu_bus,
            ppu_bus: rc_ppu_bus,
            ppu: Rc::new(ppu),
            cpu: Rc::new(cpu),
            state: Box::new(SystemState::default()),
            cartridge: rc_cartridge,
        };

        system
    }

    pub fn tick(&mut self) {
        self.cpu.tick(&mut self.state);
        self.ppu.tick(&mut self.state);
        self.ppu.tick(&mut self.state);
        self.ppu.tick(&mut self.state);
    }
}
