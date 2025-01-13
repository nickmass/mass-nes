mod app;
mod audio;
mod cpu_6502;
mod debug_state;
mod gfx;
mod gl;
mod runner;
mod widgets;

pub use app::DebuggerApp;
pub use eframe::{egui, egui_glow};
pub use widgets::{EguiMessageLayer, MessageStore};

#[cfg(target_arch = "wasm32")]
mod main;
