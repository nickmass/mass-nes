use std::rc::Rc;
use nes::bus::AddressBus;

enum Region {
    Ntsc,
    Pal,
}

struct System {
    cpu_bus: Rc<AddressBus>,
    ppu_bus: Rc<AddressBus>,
    region: Region,
}


impl System {
    fn new(region: Region) -> System {
        let cpu_bus = Rc::new(AddressBus::new());
        let ppu_bus = Rc::new(AddressBus::new());
        
        System {
            cpu_bus: cpu_bus,
            ppu_bus: ppu_bus,
            region: region,
        }
    }
}
