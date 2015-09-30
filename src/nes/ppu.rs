use std::rc::Rc;
use nes::bus::{AddressBus, AddressReader, AddressWriter, SimpleAddress};
use nes::system::Region;

pub struct Ppu {
    bus: Rc<AddressBus>,
    region: Region,
}

impl Ppu {
    pub fn new(region: Region, cpu_bus: Rc<AddressBus>, ppu_bus: Rc<AddressBus>) -> Rc<Ppu> {
        let ppu = Rc::new(Ppu {
            bus: ppu_bus.clone(),
            region: region,
        });
        
        cpu_bus.register_read(SimpleAddress::new(0x2000), ppu.clone());
        cpu_bus.register_write(SimpleAddress::new(0x2001), ppu.clone());

        ppu
    }
}

impl AddressReader for Ppu {
    fn read(&self, addr: u16) -> u8 {
        println!("PPU {}", addr);
        0
    }
}

impl AddressWriter for Ppu {
    fn write(&self, addr: u16, value: u8) {
        println!("PPU {}: {}", addr, value);
    }
}
