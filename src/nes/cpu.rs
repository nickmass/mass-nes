use std::rc::Rc;
use nes::bus::{AddressBus, AddressReader, AddressWriter, SimpleAddress};
use nes::system::SystemState;

#[derive(Default)]
pub struct CpuState {
    current_tick: u64,
}

pub struct Cpu {
    cpu_bus: Option<Rc<AddressBus>>,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {cpu_bus: None}
    }

    pub fn init(&mut self, cpu_bus: Rc<AddressBus>) {
        self.cpu_bus = Some(cpu_bus);
    }

    pub fn tick(&self,  state: &mut SystemState) {
        self.cpu_bus.as_ref().unwrap().read(state, 0x2000);
    }
}
