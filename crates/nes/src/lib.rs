mod apu;
mod bus;
mod cartridge;
mod channel;
mod cpu;
mod debug;
mod input;
mod machine;
mod mapper;
mod memory;
mod ops;
mod ppu;
mod ppu_step;
mod region;

pub use cartridge::Cartridge;
pub use machine::{Controller, Machine, UserInput};
use nes_traits::SaveState;
pub use region::Region;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData(<Machine as SaveState>::Data);
