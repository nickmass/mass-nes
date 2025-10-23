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
mod ring_buf;
pub mod run_until;

pub use apu::{ChannelPlayback, ChannelSamples};
pub use cartridge::{Cartridge, CartridgeInfo};
pub use debug::{Debug, DebugEvent, MachineState};
#[cfg(feature = "debugger")]
pub use debug::{WatchFieldName, WatchItem};
pub use input::{Controller, InputSource, SimpleInput, UserInput};
pub use machine::{Machine, RunResult};
pub use mapper::{FdsInput, MapperInput, SaveWram};
#[cfg(feature = "save-states")]
use nes_traits::SaveState;
pub use ppu::FrameEnd;
pub use region::Region;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "save-states")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData(<Machine as SaveState>::Data);
