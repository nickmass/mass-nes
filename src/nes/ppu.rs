use std::rc::Rc;
use nes::bus::{AddressBus, AddressReader, AddressWriter, SimpleAddress};
use nes::system::{Region, SystemState};

#[derive(Default)]
pub struct PpuState {
    current_tick: u64,
}

pub struct Ppu {
    region: Region,
    cpu_bus: Option<Rc<AddressBus>>,
    ppu_bus: Option<Rc<AddressBus>>,
}

impl Ppu {
    pub fn new(region: Region, cpu_bus: &mut AddressBus, ppu_bus: &mut AddressBus) -> Ppu {
        let ppu = Ppu {
            region: region,
            cpu_bus: None,
            ppu_bus: None,
        };
        
        cpu_bus.register_read::<_, Ppu>(SimpleAddress::new(0x2000));
        cpu_bus.register_write::<_, Ppu>(SimpleAddress::new(0x2001));

        ppu
    }

    pub fn init(&mut self, cpu_bus: Rc<AddressBus>, ppu_bus: Rc<AddressBus>) {
        self.cpu_bus = Some(cpu_bus);
        self.ppu_bus = Some(ppu_bus);
    }


}

impl AddressReader for Ppu {
    fn read(state: &mut SystemState, addr: u16) -> u8 {
        state.ppu.current_tick = state.ppu.current_tick + 1;
        println!("PPU {}", addr);
        0
    }
}

impl AddressWriter for Ppu {
    fn write(state: &mut SystemState, addr: u16, value: u8) {
        println!("PPU {}: {}", addr, value);
    }
}
