mod nrom;
mod uxrom;
mod sxrom;

use nes::system::{System, SystemState};
use nes::bus::{DeviceKind, BusKind};
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
}

pub fn ines(ines_number: u8, state: &mut SystemState, cart: &Cartridge) -> Box<Mapper> {
    match ines_number {
        0 => Box::new(nrom::Nrom::new(cart, state)),
        1 => Box::new(sxrom::Sxrom::new(cart, state)),
        2 => Box::new(uxrom::Uxrom::new(cart, state)),
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
        panic!("Mapper not initilized");
    }
    
    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr:u16) -> u8 {
        panic!("Mapper not initilized");
        0
    }
    
    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState,
            addr: u16) -> u8 {
        panic!("Mapper not initilized");
        0
    }
    
    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16,
             value: u8) {
        panic!("Mapper not initilized");
    
    }
}
