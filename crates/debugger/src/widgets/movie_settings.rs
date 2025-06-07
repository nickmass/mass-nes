use crate::egui;

use eframe::egui::Widget;
use egui::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSettingsState {
    pub show_settings: bool,
    pub frame_offset: i32,
    pub restore_wram: bool,
}

impl Default for MovieSettingsState {
    fn default() -> Self {
        Self {
            show_settings: false,
            frame_offset: -2,
            restore_wram: true,
        }
    }
}

pub struct MovieSettings {}

impl MovieSettings {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&self, state: &mut MovieSettingsState, ctx: &Context) -> Option<egui::Response> {
        if !state.show_settings {
            return None;
        }

        egui::Window::new("Movie Settings")
            .auto_sized()
            .show(ctx, |ui| {
                egui::Slider::new(&mut state.frame_offset, -5..=5)
                    .text("Frame Offset")
                    .integer()
                    .show_value(true)
                    .ui(ui);

                ui.checkbox(&mut state.restore_wram, "Restore WRAM Save");

                ui.horizontal(|ui| {
                    if ui.button("Defaults").clicked() {
                        *state = MovieSettingsState {
                            show_settings: true,
                            ..Default::default()
                        };
                    }

                    let res = ui.button("Ok");
                    if res.clicked() {
                        state.show_settings = false;
                    }
                    res
                })
                .inner
            })
            .and_then(|res| res.inner)
    }
}
