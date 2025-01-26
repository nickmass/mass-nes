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

pub use cartridge::{Cartridge, CartridgeInfo};
pub use debug::MachineState;
pub use machine::{Controller, FdsInput, Machine, MapperInput, RunResult, UserInput};
pub use mapper::SaveWram;
#[cfg(feature = "save-states")]
use nes_traits::SaveState;
pub use region::Region;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "save-states")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData(<Machine as SaveState>::Data);
