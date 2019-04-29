mod action53;
mod axrom;
mod bf909x;
mod cnrom;
mod fme7;
mod nrom;
mod sxrom;
mod txrom;
mod uxrom;

use crate::bus::{AddressBus, BusKind};
use crate::cartridge::Cartridge;
use crate::ppu::Ppu;
use crate::system::{System, SystemState};

pub trait Mapper {
    fn register(
        &self,
        state: &mut SystemState,
        cpu: &mut AddressBus,
        ppu: &mut Ppu,
        cart: &Cartridge,
    );
    fn peek(&self, bus: BusKind, system: &System, state: &SystemState, addr: u16) -> u8;
    fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, addr: u16, value: u8);
    fn get_irq(&mut self) -> bool {
        false
    }
    fn tick(&self, system: &System, state: &mut SystemState);
    fn nt_peek(&self, system: &System, state: &SystemState, addr: u16) -> u8;
    fn nt_read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn nt_write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8);
    fn update_ppu_addr(&self, system: &System, state: &mut SystemState, addr: u16);
}

pub fn ines(ines_number: u8, state: &mut SystemState, cart: &Cartridge) -> Box<dyn Mapper> {
    match ines_number {
        0 => Box::new(nrom::Nrom::new(cart, state)),
        1 | 65 => Box::new(sxrom::Sxrom::new(cart, state)),
        2 => Box::new(uxrom::Uxrom::new(cart, state)),
        3 => Box::new(cnrom::Cnrom::new(cart, state)),
        4 => Box::new(txrom::Txrom::new(cart, state)),
        7 => Box::new(axrom::Axrom::new(cart, state)),
        28 => Box::new(action53::Action53::new(cart, state)),
        69 => Box::new(fme7::Fme7::new(cart, state)),
        71 | 232 => Box::new(bf909x::Bf909x::new(cart, state)),
        _ => {
            println!("Mapper not implemented.");
            Box::new(nrom::Nrom::new(cart, state))
        }
    }
}
