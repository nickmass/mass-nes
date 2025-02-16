mod action53;
mod axrom;
mod bf909x;
mod bxrom;
mod cnrom;
mod fds;
mod fme7;
mod mmc1;
mod mmc2;
mod mmc3;
mod mmc5;
mod namco163;
mod nina001;
mod nrom;
mod uxrom;
mod vrc4;
mod vrc6;
mod vrc_irq;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, BusKind};
use crate::cartridge::{Fds, INes};
use crate::debug::Debug;
use crate::machine::MapperInput;
use crate::ppu::PpuFetchKind;

use std::cell::RefCell;
use std::rc::Rc;

pub use traits::MapperState;

#[derive(Debug, Clone)]
pub struct SaveWram(Vec<u8>);

impl SaveWram {
    pub fn from_bytes<B: ToOwned<Owned = Vec<u8>>>(bytes: B) -> Self {
        Self(bytes.to_owned())
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.0
    }
}

#[cfg(feature = "save-states")]
mod traits {
    use super::{Mapper, RcMapper};
    use nes_traits::{BinarySaveState, SaveState};

    pub trait MapperState: Mapper + BinarySaveState {}
    impl<T: Mapper + BinarySaveState> MapperState for T {}

    impl SaveState for RcMapper {
        type Data = Vec<u8>;

        fn save_state(&self) -> Self::Data {
            self.0.borrow().binary_save_state()
        }

        fn restore_state(&mut self, state: &Self::Data) {
            self.0.borrow_mut().binary_restore_state(state.as_slice());
        }
    }
}

#[cfg(not(feature = "save-states"))]
mod traits {
    pub trait MapperState: super::Mapper {}
    impl<T: super::Mapper> MapperState for T {}
}

pub trait Mapper {
    fn register(&self, cpu: &mut AddressBus);

    fn peek(&self, bus: BusKind, addr: u16) -> u8;

    fn read(&mut self, bus: BusKind, addr: u16) -> u8;

    fn write(&mut self, bus: BusKind, addr: u16, value: u8);

    fn get_irq(&mut self) -> bool {
        false
    }

    fn tick(&mut self) {}

    fn peek_ppu_fetch(&self, address: u16, kind: PpuFetchKind) -> Nametable;

    fn ppu_fetch(&mut self, address: u16, kind: PpuFetchKind) -> Nametable {
        self.peek_ppu_fetch(address, kind)
    }

    fn get_sample(&self) -> Option<i16> {
        None
    }

    fn input(&mut self, _input: MapperInput) {}

    fn save_wram(&self) -> Option<SaveWram> {
        None
    }
}

#[derive(Clone)]
pub struct RcMapper(Rc<RefCell<dyn MapperState>>);

impl RcMapper {
    fn new<T: MapperState + 'static>(mapper: T) -> Self {
        RcMapper(Rc::new(RefCell::new(mapper)))
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        self.0.borrow().register(cpu)
    }

    pub fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        self.0.borrow().peek(bus, addr)
    }

    pub fn read(&self, bus: BusKind, addr: u16) -> u8 {
        self.0.borrow_mut().read(bus, addr)
    }

    pub fn write(&self, bus: BusKind, addr: u16, value: u8) {
        self.0.borrow_mut().write(bus, addr, value)
    }

    pub fn peek_ppu_fetch(&self, address: u16, kind: PpuFetchKind) -> Nametable {
        self.0.borrow_mut().peek_ppu_fetch(address, kind)
    }

    pub fn ppu_fetch(&self, address: u16, kind: PpuFetchKind) -> Nametable {
        self.0.borrow_mut().ppu_fetch(address, kind)
    }

    pub fn get_irq(&self) -> bool {
        self.0.borrow_mut().get_irq()
    }

    pub fn tick(&self) {
        self.0.borrow_mut().tick()
    }

    pub fn get_sample(&self) -> Option<i16> {
        self.0.borrow().get_sample()
    }

    pub fn input(&self, input: MapperInput) {
        self.0.borrow_mut().input(input);
    }

    pub fn save_wram(&self) -> Option<SaveWram> {
        self.0.borrow().save_wram()
    }
}

pub fn ines(cart: INes, debug: Rc<Debug>) -> RcMapper {
    match cart.mapper {
        0 => RcMapper::new(nrom::Nrom::new(cart)),
        1 | 65 => RcMapper::new(mmc1::Mmc1::new(cart)),
        2 => RcMapper::new(uxrom::Uxrom::new(cart)),
        3 => RcMapper::new(cnrom::Cnrom::new(cart)),
        4 => RcMapper::new(mmc3::Mmc3::new(cart, debug)),
        5 => RcMapper::new(mmc5::Mmc5::new(cart, debug)),
        7 => RcMapper::new(axrom::Axrom::new(cart)),
        9 => RcMapper::new(mmc2::Mmc2::new(cart)),
        19 => RcMapper::new(namco163::Namco163::new(cart, debug)),
        21 => match cart.submapper {
            Some(2) => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4c, debug)),
            _ => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4a, debug)),
        },
        22 => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2a, debug)),
        23 => match cart.submapper {
            Some(2) => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4e, debug)),
            Some(3) => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2b, debug)),
            _ => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4f, debug)),
        },
        24 => RcMapper::new(vrc6::Vrc6::new(cart, vrc6::Vrc6Variant::A, debug)),
        25 => match cart.submapper {
            Some(2) => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4d, debug)),
            Some(3) => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2c, debug)),
            _ => RcMapper::new(vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4b, debug)),
        },
        26 => RcMapper::new(vrc6::Vrc6::new(cart, vrc6::Vrc6Variant::B, debug)),
        28 => RcMapper::new(action53::Action53::new(cart)),
        34 => match cart.submapper.unwrap_or_default() {
            1 => RcMapper::new(nina001::Nina001::new(cart)),
            2 => RcMapper::new(bxrom::Bxrom::new(cart)),
            0 | _ => {
                if cart.chr_rom.len() > 8 * 1024 || cart.chr_ram_bytes == 0 {
                    RcMapper::new(nina001::Nina001::new(cart))
                } else {
                    RcMapper::new(bxrom::Bxrom::new(cart))
                }
            }
        },
        69 => RcMapper::new(fme7::Fme7::new(cart)),
        71 | 232 => RcMapper::new(bf909x::Bf909x::new(cart)),
        206 => {
            tracing::warn!("limited mapper support");
            RcMapper::new(mmc3::Mmc3::new(cart, debug))
        }
        _ => {
            tracing::error!("mapper not implemented");
            RcMapper::new(nrom::Nrom::new(cart))
        }
    }
}

pub fn fds(disk: Fds) -> RcMapper {
    RcMapper::new(fds::Fds::new(disk))
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    Single(Nametable),
    Custom,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Nametable {
    InternalA,
    InternalB,
    External,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
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

    pub fn set(&self, mirroring: Mirroring) {
        self.mirroring.set(mirroring);
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
