use std::rc::Rc;
use nes::bus::AddressBus;
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};

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
    state: SystemState,
}


impl System {
    pub fn new(region: Region) -> System {
        let mut cpu_bus = AddressBus::new();
        let mut ppu_bus = AddressBus::new();
        
        let mut cpu = Cpu::new();
        let mut ppu = Ppu::new(Region::Ntsc ,&mut cpu_bus,&mut ppu_bus);
       
        let rc_cpu_bus = Rc::new(cpu_bus);
        let rc_ppu_bus = Rc::new(ppu_bus);

        cpu.init(rc_cpu_bus.clone());
        ppu.init(rc_cpu_bus.clone(), rc_ppu_bus.clone());
        
        let system = System {
            region: region,
            cpu_bus: rc_cpu_bus,
            ppu_bus: rc_ppu_bus,
            ppu: Rc::new(ppu),
            cpu: Rc::new(cpu),
            state: SystemState::default()
        };

        system
    }

    pub fn tick(&mut self) {
        self.cpu.tick(&self.cpu_bus, &mut self.state);
    }
}
