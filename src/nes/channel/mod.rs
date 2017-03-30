mod pulse;
mod noise;
mod triangle;
mod dmc;

pub use self::pulse::{Pulse, PulseChannel};
pub use self::noise::Noise;
pub use self::triangle::Triangle;
pub use self::dmc::Dmc;

use system::{System, SystemState};
use cpu::Cpu;

pub trait Channel {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu);
    fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8);
    fn tick(&self, system: &System, state: &mut SystemState) -> u8;
    fn enable(&self);
    fn disable(&self);
    fn get_state(&self) -> bool; 
}

