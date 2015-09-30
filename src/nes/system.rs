use std::rc::Rc;
use nes::bus::AddressBus;
use nes::ppu::Ppu;

pub enum Region {
    Ntsc,
    Pal,
}

struct System {
    region: Region,
    cpu_bus: Rc<AddressBus>,
    ppu_bus: Rc<AddressBus>,
    ppu: Rc<Ppu>,
}


impl System {
    fn new(region: Region) -> System {
        let cpu_bus = Rc::new(AddressBus::new());
        let ppu_bus = Rc::new(AddressBus::new());
        
        let ppu = Ppu::new(Region::Ntsc , cpu_bus.clone(), ppu_bus.clone());

        System {
            region: region,
            cpu_bus: cpu_bus,
            ppu_bus: ppu_bus,
            ppu: ppu,
        }
    }
}
