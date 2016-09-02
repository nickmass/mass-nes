mod nrom;
mod sxrom;
mod uxrom;
mod cnrom;
mod axrom;
mod txrom;

use nes::system::{System, SystemState};
use nes::bus::BusKind;
use nes::cpu::Cpu;
use nes::ppu::Ppu;
use nes::cartridge::Cartridge;

pub trait Mapper { 
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
                cart: &Cartridge);
    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr:u16) -> u8;
    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState,
            addr: u16) -> u8;
    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16,
             value: u8);
    fn tick(&self, system: &System, state: &mut SystemState);
    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8;
    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, 
                value: u8);
}

pub fn ines(ines_number: u8, state: &mut SystemState, cart: &Cartridge) -> Box<Mapper> {
    match ines_number {
        0 => Box::new(nrom::Nrom::new(cart, state)),
        1 => Box::new(sxrom::Sxrom::new(cart, state)),
        2 => Box::new(uxrom::Uxrom::new(cart, state)),
        3 => Box::new(cnrom::Cnrom::new(cart, state)),
        4 => Box::new(txrom::Txrom::new(cart,state)),
        7 => Box::new(axrom::Axrom::new(cart, state)),
        _ => {
            println!("Mapper not implemented.");
            Box::new(nrom::Nrom::new(cart, state))
        }
    }
}

pub struct Null;

impl Mapper for Null { 
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu, ppu: &mut Ppu,
                cart: &Cartridge) {
        panic!("Mapper not initialized");
    }
    
    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr:u16) -> u8 {
        panic!("Mapper not initialized");
    }
    
    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState,
            addr: u16) -> u8 {
        panic!("Mapper not initialized");
    }
    
    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16,
             value: u8) {
        panic!("Mapper not initialized");
    
    }

    fn tick(&self, system: &System, state: &mut SystemState) {
        panic!("Mapper not initialized");
    }

    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        system.ppu.nametables.read(state, addr)
    }

    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        system.ppu.nametables.write(state, addr, value);
    }
}
