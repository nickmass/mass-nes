use crate::egui;
use egui::Vec2;

use crate::debug_state::PpuView;

pub struct PaletteViewer<'a> {
    ppu: PpuView<'a>,
}

impl<'a> PaletteViewer<'a> {
    pub fn new(ppu: PpuView<'a>) -> Self {
        Self { ppu }
    }

    pub fn ui(&self, selected_palette: &mut u8, ui: &mut egui::Ui) {
        let selection_color = ui.visuals().selection.stroke.color;

        ui.vertical(|ui| {
            for kind in 0..2 {
                ui.horizontal(|ui| {
                    for pal in 0..4 {
                        let palette_id = pal + (kind * 4);
                        let color = if *selected_palette == palette_id {
                            selection_color
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let mut frame = egui::Frame::none().fill(color).inner_margin(2.0).begin(ui);
                        frame.content_ui.spacing_mut().item_spacing = Vec2::ZERO;

                        for idx in 0..4 {
                            let idx = idx + (kind * 16) + (pal * 4);

                            let color = self.ppu.pal_entry_color(idx);

                            let (rect, _) = frame
                                .content_ui
                                .allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                            frame.content_ui.painter().rect_filled(rect, 0.0, color);
                        }

                        let res = frame.allocate_space(ui);
                        let res = res.union(frame.content_ui.interact(
                            frame.content_ui.min_rect(),
                            egui::Id::new(palette_id),
                            egui::Sense::click(),
                        ));

                        if res.clicked() {
                            *selected_palette = palette_id;
                        }

                        if res.hovered() {
                            frame.frame.fill = selection_color;
                        }

                        frame.paint(ui);
                    }
                });
            }
        });
    }
}
