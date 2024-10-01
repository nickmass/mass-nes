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
mod ppu;
mod ppu_step;
mod region;

pub use cartridge::Cartridge;
pub use machine::{Controller, Machine, UserInput};
#[cfg(feature = "save-states")]
use nes_traits::SaveState;
pub use region::Region;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "save-states")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData(<Machine as SaveState>::Data);
