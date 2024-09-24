mod dmc;
mod noise;
mod pulse;
mod triangle;

pub use self::dmc::Dmc;
pub use self::noise::Noise;
pub use self::pulse::{Pulse, PulseChannel};
pub use self::triangle::Triangle;

use crate::apu::ApuSnapshot;
use crate::bus::AddressBus;

pub trait Channel {
    fn register(&self, cpu: &mut AddressBus);
    fn write(&mut self, addr: u16, value: u8);
    fn tick(&mut self, state: ApuSnapshot) -> u8;
    fn enable(&mut self);
    fn disable(&mut self);
    fn get_state(&self) -> bool;
}
