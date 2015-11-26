struct NRom;
use std::rc::Rc;
use super::bus::AddressBus;

impl NRom {
    pub fn new(cpu_bus: &mut AddressBus, ppu_bus: &mut AddressBus) {
    }

    pub fn init(&mut self, cpu_bus: Rc<AddressBus>, ppu_bus: Rc<AddressBus>) {
    }
}
