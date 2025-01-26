mod app;
mod cpu_6502;
mod debug_state;
mod gfx;
mod gl;
mod platform;
mod runner;
mod spawner;
mod widgets;

pub use app::DebuggerApp;
pub use eframe::{egui, egui_glow};
pub use widgets::{EguiMessageLayer, MessageStore};

// used for window title and save data directory
pub const APP_NAME: &str = "Mass Emu";

#[cfg(target_arch = "wasm32")]
mod main;
