pub mod pulse;

use nes::system::{System, SystemState};
use nes::cpu::Cpu;

pub trait Channel {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu);
    fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8;
    fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8);
    fn tick(&self, system: &System, state: &mut SystemState) -> u8;
    fn enable(&self, system: &System, state: &mut SystemState);
    fn disable(&self, system: &System, state: &mut SystemState);
    fn get_state(&self, system: &System, state: &mut SystemState) -> bool; 
}

