use crate::egui;

use crate::debug_state::DebugUiState;

use std::collections::HashSet;

use egui::Vec2;
use serde::{Deserialize, Serialize};

pub struct VariableViewer {
    group_stack: Vec<bool>,
}

impl VariableViewer {
    pub fn new() -> Self {
        Self {
            group_stack: Vec::new(),
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        state: &mut VariableViewerState,
        debug: &DebugUiState,
    ) {
        egui::Window::new("Variables").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(&mut state.display_hex, "0x");
            });
            ui.separator();
            egui::Grid::new("variable_grid")
                .min_col_width(15.0)
                .num_columns(3)
                .show(ui, |ui| {
                    self.group_stack.clear();
                    let items = debug.watch_items();
                    for v in items {
                        let expand = self.group_stack.last().copied().unwrap_or(true);
                        match v {
                            nes::WatchItem::EndGroup => {
                                self.group_stack.pop();
                                continue;
                            }
                            nes::WatchItem::Group(_) if !expand => {
                                self.group_stack.push(false);
                                continue;
                            }
                            _ if !expand => {
                                continue;
                            }
                            nes::WatchItem::Group(name) => {
                                let (_rect, toggle) =
                                    ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::click());
                                let open = if state.expand_groups.contains(*name) {
                                    if toggle.clicked() {
                                        state.expand_groups.remove(*name);
                                    }
                                    true
                                } else {
                                    if toggle.clicked() {
                                        state.expand_groups.insert(name.to_string());
                                    }
                                    false
                                };

                                let openness = if open { 1.0 } else { 0.0 };

                                egui::collapsing_header::paint_default_icon(ui, openness, &toggle);

                                let label = egui::Label::new(egui::RichText::new(*name).strong())
                                    .selectable(false)
                                    .sense(egui::Sense::click());

                                if ui.add(label).clicked() {
                                    if open {
                                        state.expand_groups.remove(*name);
                                    } else {
                                        state.expand_groups.insert(name.to_string());
                                    }
                                }
                                self.group_stack.push(open);
                            }
                            nes::WatchItem::Field(name, value) if state.display_hex => {
                                ui.allocate_space(Vec2::splat(1.0));
                                ui.label(*name);
                                ui.label(egui::RichText::new(format!("{:#X}", value)).monospace());
                            }
                            nes::WatchItem::Field(name, value) => {
                                ui.allocate_space(Vec2::splat(1.0));
                                ui.label(*name);
                                ui.label(egui::RichText::new(format!("{}", value)).monospace());
                            }
                        }
                        ui.end_row();
                    }
                })
        });
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default = "Default::default")]
pub struct VariableViewerState {
    display_hex: bool,
    expand_groups: HashSet<String>,
}
