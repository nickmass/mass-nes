use std::rc::Rc;
use nes::bus::{AddressValidator, AddressBus, BusKind, DeviceKind, Address};
use nes::system::{Region, SystemState, System};

#[derive(Default)]
pub struct PpuState {
    current_tick: u64,
}

pub struct Ppu {
    region: Region,
    bus: AddressBus,
}

impl Ppu {
    pub fn new(region: Region) -> Ppu {
        let ppu = Ppu {
            region: region,
            bus: AddressBus::new(BusKind::Ppu),
        };

        ppu
    }

    pub fn register_read<T>(&mut self, device: DeviceKind, addr: T) where T: AddressValidator {
        self.bus.register_read(device, addr);
    }

    pub fn register_write<T>(&mut self, device: DeviceKind, addr: T) where T: AddressValidator {
        self.bus.register_write(device, addr);
    }

    pub fn read(&self, bus: BusKind, state: &mut SystemState, address: u16) -> u8 {
        println!("IN PPU");
        0
    }

    pub fn write(&self, bus: BusKind,  state: &mut SystemState, address: u16, value: u8) {

    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.ppu.current_tick += 1;
    }
}
