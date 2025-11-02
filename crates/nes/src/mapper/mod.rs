mod action53;
mod axrom;
mod bf909x;
mod bxrom;
mod cnrom;
mod color_dreams;
mod fds;
mod fme7;
mod game_genie;
mod gxrom;
mod j87;
mod mapper_031;
mod mmc1;
mod mmc2;
mod mmc3;
mod mmc5;
mod namco163;
mod namco175_340;
mod nina001;
mod nina006;
mod nrom;
mod nsf;
mod rainbow;
mod uxrom;
mod vrc4;
mod vrc6;
mod vrc7;
mod vrc_irq;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::Region;
use crate::bus::{AddressBus, BusKind};
use crate::cartridge::{Fds, INes, NsfFile};
use crate::debug::Debug;
use crate::memory::Memory;
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

    pub trait MapperState: Mapper + BinarySaveState {
        fn rc(self) -> super::RcMapper
        where
            Self: Sized + 'static,
        {
            super::RcMapper::new(self)
        }
    }
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
    pub trait MapperState: super::Mapper {
        fn rc(self) -> super::RcMapper
        where
            Self: Sized + 'static,
        {
            super::RcMapper::new(self)
        }
    }
    impl<T: super::Mapper> MapperState for T {}
}

pub trait Mapper {
    fn register(&self, cpu: &mut AddressBus);

    fn peek(&self, bus: BusKind, addr: u16) -> u8;

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
        self.peek(bus, addr)
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8);

    fn get_irq(&self) -> bool {
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

    fn power(&mut self) {}

    fn input(&mut self, _input: MapperInput) {}

    fn save_wram(&self) -> Option<SaveWram> {
        None
    }

    #[cfg(feature = "debugger")]
    fn watch(&self, visitor: &mut crate::debug::WatchVisitor) {
        let mut mapper = visitor.group("Mapper");
        mapper.value("IRQ", self.get_irq());
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
        self.0.borrow().get_irq()
    }

    pub fn tick(&self) {
        self.0.borrow_mut().tick()
    }

    pub fn get_sample(&self) -> Option<i16> {
        self.0.borrow().get_sample()
    }

    pub fn power(&self) {
        self.0.borrow_mut().power()
    }

    pub fn input(&self, input: MapperInput) {
        self.0.borrow_mut().input(input);
    }

    pub fn save_wram(&self) -> Option<SaveWram> {
        self.0.borrow().save_wram()
    }

    #[cfg(feature = "debugger")]
    pub fn watch(&self, visitor: &mut crate::debug::WatchVisitor) {
        self.0.borrow().watch(visitor);
    }

    pub fn with_game_genie(self) -> Self {
        game_genie::GameGenie::new(self).rc()
    }
}

pub fn ines(cart: INes, debug: Rc<Debug>) -> RcMapper {
    match cart.mapper {
        0 => nrom::Nrom::new(cart).rc(),
        1 | 65 => mmc1::Mmc1::new(cart).rc(),
        2 => uxrom::Uxrom::new(cart).rc(),
        3 => cnrom::Cnrom::new(cart).rc(),
        148 => {
            tracing::warn!("limited mapper support");
            cnrom::Cnrom::new(cart).rc()
        }
        4 => match cart.submapper {
            Some(1) => mmc3::Mmc3::new(cart, mmc3::Mmc3Variant::Mmc6, debug).rc(),
            Some(4) => mmc3::Mmc3::new(cart, mmc3::Mmc3Variant::Mmc3AltIrq, debug).rc(),
            _ => mmc3::Mmc3::new(cart, mmc3::Mmc3Variant::Mmc3, debug).rc(),
        },
        5 => mmc5::Mmc5::new(cart, debug).rc(),
        7 => axrom::Axrom::new(cart).rc(),
        9 => mmc2::Mmc2::new(cart, mmc2::Mmc2Variant::Mmc2).rc(),
        10 => mmc2::Mmc2::new(cart, mmc2::Mmc2Variant::Mmc4).rc(),
        11 => color_dreams::ColorDreams::new(cart).rc(),
        19 => namco163::Namco163::new(cart, debug).rc(),
        21 => match cart.submapper {
            Some(2) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4c, debug).rc(),
            _ => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4a, debug).rc(),
        },
        22 => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2a, debug).rc(),
        23 => match cart.submapper {
            Some(1) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4f, debug).rc(),
            Some(2) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4e, debug).rc(),
            Some(3) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2b, debug).rc(),
            _ => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2b, debug).rc(),
        },
        24 => vrc6::Vrc6::new(cart, vrc6::Vrc6Variant::A, debug).rc(),
        25 => match cart.submapper {
            Some(2) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4d, debug).rc(),
            Some(3) => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc2c, debug).rc(),
            _ => vrc4::Vrc4::new(cart, vrc4::Vrc4Variant::Vrc4b, debug).rc(),
        },
        26 => vrc6::Vrc6::new(cart, vrc6::Vrc6Variant::B, debug).rc(),
        28 => action53::Action53::new(cart).rc(),
        31 => mapper_031::Mapper031::new(cart).rc(),
        34 => match cart.submapper.unwrap_or_default() {
            1 => nina001::Nina001::new(cart).rc(),
            2 => bxrom::Bxrom::new(cart).rc(),
            0 | _ => {
                if cart.chr_rom.len() > 8 * 1024 || cart.chr_ram_bytes == 0 {
                    nina001::Nina001::new(cart).rc()
                } else {
                    bxrom::Bxrom::new(cart).rc()
                }
            }
        },
        66 => gxrom::Gxrom::new(cart).rc(),
        69 => fme7::Fme7::new(cart, debug).rc(),
        71 | 232 => bf909x::Bf909x::new(cart).rc(),
        79 | 146 => nina006::Nina006::new(cart).rc(),
        85 => match cart.submapper {
            Some(1) => vrc7::Vrc7::new(cart, vrc7::Vrc7Variant::Vrc7b, debug).rc(),
            Some(2) => vrc7::Vrc7::new(cart, vrc7::Vrc7Variant::Vrc7a, debug).rc(),
            _ => vrc7::Vrc7::new(cart, vrc7::Vrc7Variant::Undefined, debug).rc(),
        },
        87 => j87::J87::new(cart).rc(),
        206 => {
            tracing::warn!("limited mapper support");
            mmc3::Mmc3::new(cart, mmc3::Mmc3Variant::Mmc3, debug).rc()
        }
        210 => match cart.submapper {
            Some(1) => {
                namco175_340::Namco175_340::new(cart, namco175_340::NamcoVariant::Namco175).rc()
            }
            Some(2) => {
                namco175_340::Namco175_340::new(cart, namco175_340::NamcoVariant::Namco340).rc()
            }
            _ => {
                tracing::warn!("iNES 210 rom unknown sub-mapper");
                namco175_340::Namco175_340::new(cart, namco175_340::NamcoVariant::Unspecified).rc()
            }
        },
        682 | 3871 => {
            tracing::warn!("limited mapper support");
            rainbow::Rainbow::new(cart, debug).rc()
        }
        _ => {
            tracing::error!("mapper not implemented");
            nrom::Nrom::new(cart).rc()
        }
    }
}

pub fn fds(disk: Fds) -> RcMapper {
    fds::Fds::new(disk).rc()
}

pub fn nsf(region: Region, data: NsfFile) -> RcMapper {
    nsf::Nsf::new(region, data).rc()
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    Single(Nametable),
    FourScreen,
}

#[derive(Debug, Copy, Clone)]
pub enum MapperInput {
    Fds(FdsInput),
}

#[derive(Debug, Copy, Clone)]
pub enum FdsInput {
    SetDisk(Option<usize>),
}

impl Mirroring {
    fn ppu_fetch(&self, address: u16) -> Nametable {
        if address & 0x2000 != 0 {
            match self {
                Mirroring::Single(n) => *n,
                Mirroring::Horizontal if address & 0x800 != 0 => Nametable::InternalA,
                Mirroring::Horizontal => Nametable::InternalB,
                Mirroring::Vertical if address & 0x400 != 0 => Nametable::InternalA,
                Mirroring::Vertical => Nametable::InternalB,
                Mirroring::FourScreen => match address & 0xc00 {
                    0x000 => Nametable::InternalA,
                    0x400 => Nametable::InternalB,
                    _ => Nametable::External,
                },
            }
        } else {
            Nametable::External
        }
    }
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
    mirroring: Mirroring,
}

impl SimpleMirroring {
    pub fn new<T: Into<Mirroring>>(mirroring: T) -> Self {
        Self {
            mirroring: mirroring.into(),
        }
    }

    pub fn internal_a(&mut self) {
        self.mirroring = Mirroring::Single(Nametable::InternalA);
    }

    pub fn internal_b(&mut self) {
        self.mirroring = Mirroring::Single(Nametable::InternalB);
    }

    pub fn horizontal(&mut self) {
        self.mirroring = Mirroring::Horizontal;
    }

    pub fn vertical(&mut self) {
        self.mirroring = Mirroring::Vertical;
    }

    pub fn set(&mut self, mirroring: Mirroring) {
        self.mirroring = mirroring;
    }

    pub fn ppu_fetch(&self, address: u16) -> Nametable {
        self.mirroring.ppu_fetch(address)
    }
}
