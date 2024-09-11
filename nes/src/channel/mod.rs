mod dmc;
mod noise;
mod pulse;
mod triangle;

pub use self::dmc::Dmc;
pub use self::noise::Noise;
pub use self::pulse::{Pulse, PulseChannel};
pub use self::triangle::Triangle;

use crate::apu::ApuState;
use crate::bus::AddressBus;

pub trait Channel {
    fn register(&self, cpu: &mut AddressBus);
    fn write(&self, addr: u16, value: u8);
    fn tick(&self, state: &ApuState) -> u8;
    fn enable(&self);
    fn disable(&self);
    fn get_state(&self) -> bool;
}
