mod dmc;
mod noise;
mod pulse;
mod triangle;

pub use self::dmc::Dmc;
pub use self::noise::Noise;
pub use self::pulse::{Pulse, PulseChannel};
pub use self::triangle::Triangle;

use crate::bus::AddressBus;
use crate::system::{System, SystemState};

pub trait Channel {
    fn register(&self, state: &mut SystemState, cpu: &mut AddressBus);
    fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8);
    fn tick(&self, system: &System, state: &mut SystemState) -> u8;
    fn enable(&self);
    fn disable(&self);
    fn get_state(&self) -> bool;
}
