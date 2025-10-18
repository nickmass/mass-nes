use crate::egui;

use eframe::egui::Widget;
use egui::Context;
use serde::{Deserialize, Serialize};
use ui::movie::SubframeMode as UiSubframeMode;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubframeMode {
    On,
    Off,
    Auto,
}

impl Default for SubframeMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl From<SubframeMode> for UiSubframeMode {
    fn from(value: SubframeMode) -> Self {
        match value {
            SubframeMode::On => UiSubframeMode::On,
            SubframeMode::Off => UiSubframeMode::Off,
            SubframeMode::Auto => UiSubframeMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSettingsState {
    pub show_settings: bool,
    pub frame_offset: i32,
    pub restore_wram: bool,
    pub subframe: SubframeMode,
}

impl Default for MovieSettingsState {
    fn default() -> Self {
        Self {
            show_settings: false,
            frame_offset: 0,
            restore_wram: true,
            subframe: SubframeMode::default(),
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
                    ui.label("Subframe Input Mode:");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut state.subframe, SubframeMode::Auto, "Auto");
                        ui.radio_value(&mut state.subframe, SubframeMode::On, "On");
                        ui.radio_value(&mut state.subframe, SubframeMode::Off, "Off");
                    });
                });

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
