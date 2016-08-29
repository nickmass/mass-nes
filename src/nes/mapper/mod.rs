mod nrom;
mod uxrom;

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
        2 => Box::new(uxrom::Uxrom::new(cart, state)),
        _ => {
            println!("Mapper not implemented.");
            Box::new(nrom::Nrom::new(cart, state))
        }
    }
}

#[derive(Copy, Clone)]
pub struct Bank {
    start: usize,
    end: usize,
}

pub struct Banks {
    banks: Vec<Bank>,
    size: usize,
}

impl Banks {
    pub fn load(data: &[u8], size_kb: usize) -> Banks {
        let mut v = Vec::new();
        let mut x = 0;
        while x + (size_kb * 0x400) < data.len() {
            let start = x;
            x += size_kb * 0x400;
            let end = x;
            v.push(Bank { start: start, end: end });
        }
        Banks {
            banks: v,
            size: size_kb
        }
    }

    pub fn last(&self, data: &[u8], addr: i16) -> u8 {
        let bank  = self.banks.last().unwrap();
        data[bank.start + addr as usize]
    }

    pub fn read(&self, data: &[u8], bank: usize, addr: u16) -> u8 {
        let bank = self.banks[bank % self.banks.len()];
        data[bank.start + addr as usize]
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
