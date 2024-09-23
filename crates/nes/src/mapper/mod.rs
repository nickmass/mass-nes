mod action53;
mod axrom;
mod bf909x;
mod cnrom;
mod fme7;
mod nrom;
mod pxrom;
mod sxrom;
mod txrom;
mod uxrom;

use crate::bus::{AddressBus, BusKind};
use crate::cartridge::Cartridge;

use std::rc::Rc;

pub trait Mapper {
    fn register(&self, cpu: &mut AddressBus);

    fn peek(&self, bus: BusKind, addr: u16) -> u8;

    fn read(&self, bus: BusKind, addr: u16) -> u8;

    fn write(&self, bus: BusKind, addr: u16, value: u8);

    fn get_irq(&self) -> bool {
        false
    }

    fn tick(&self) {}

    fn update_ppu_addr(&self, _addr: u16) {}

    fn ppu_fetch(&self, address: u16) -> Nametable;
}

pub fn ines(ines_number: u8, cart: Cartridge) -> Rc<dyn Mapper> {
    match ines_number {
        0 => Rc::new(nrom::Nrom::new(cart)),
        1 | 65 => Rc::new(sxrom::Sxrom::new(cart)),
        2 => Rc::new(uxrom::Uxrom::new(cart)),
        3 => Rc::new(cnrom::Cnrom::new(cart)),
        4 => Rc::new(txrom::Txrom::new(cart)),
        7 => Rc::new(axrom::Axrom::new(cart)),
        9 => Rc::new(pxrom::Pxrom::new(cart)),
        28 => Rc::new(action53::Action53::new(cart)),
        69 => Rc::new(fme7::Fme7::new(cart)),
        71 | 232 => Rc::new(bf909x::Bf909x::new(cart)),
        _ => {
            tracing::error!("mapper not implemented");
            Rc::new(nrom::Nrom::new(cart))
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    Single(Nametable),
    Custom,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Nametable {
    InternalA,
    InternalB,
    External,
}

pub struct SimpleMirroring {
    mirroring: std::cell::Cell<Mirroring>,
}

impl SimpleMirroring {
    pub fn new(mirroring: Mirroring) -> Self {
        Self {
            mirroring: std::cell::Cell::new(mirroring),
        }
    }

    pub fn internal_a(&self) {
        self.mirroring.set(Mirroring::Single(Nametable::InternalA));
    }

    pub fn internal_b(&self) {
        self.mirroring.set(Mirroring::Single(Nametable::InternalB));
    }

    pub fn horizontal(&self) {
        self.mirroring.set(Mirroring::Horizontal);
    }

    pub fn vertical(&self) {
        self.mirroring.set(Mirroring::Vertical);
    }

    pub fn ppu_fetch(&self, address: u16) -> Nametable {
        if address & 0x2000 != 0 {
            match self.mirroring.get() {
                Mirroring::Single(n) => n,
                Mirroring::Horizontal if address & 0x800 != 0 => Nametable::InternalA,
                Mirroring::Horizontal => Nametable::InternalB,
                Mirroring::Vertical if address & 0x400 != 0 => Nametable::InternalA,
                Mirroring::Vertical => Nametable::InternalB,
                Mirroring::Custom => Nametable::External,
            }
        } else {
            Nametable::External
        }
    }
}
