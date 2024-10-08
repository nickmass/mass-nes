use std::sync::{Arc, Mutex};

use crate::{egui, egui_glow, gfx::Gfx};
use egui::Vec2;

use ui::filters::Filter;

pub struct NesScreen<F: Filter> {
    default_size: Vec2,
    gfx: Arc<Mutex<Gfx<F>>>,
}

impl<F: Filter + Send + Sync + 'static> NesScreen<F> {
    pub fn new(gfx: Gfx<F>) -> Self {
        let gfx = Arc::new(Mutex::new(gfx));
        let (width, height) = gfx.lock().unwrap().filter_dimensions();
        let default_size = Vec2::new(width as f32, height as f32);

        Self { gfx, default_size }
    }

    fn paint(&self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let ratio = avail / self.default_size;

        let size = if ratio.x > ratio.y {
            let y = self.default_size.x / self.default_size.y;
            self.default_size * Vec2::new(ratio.x, ratio.x * y)
        } else {
            let x = self.default_size.y / self.default_size.x;
            self.default_size * Vec2::new(ratio.y * x, ratio.y)
        };

        let (rect, _res) = ui.allocate_exact_size(size, egui::Sense::focusable_noninteractive());

        let gfx = self.gfx.clone();
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |paint_info, _painter| {
                let mut gfx = gfx.lock().unwrap();
                gfx.render(paint_info);
            })),
        };

        ui.painter().add(callback);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<egui::Response> {
        let res = egui::Window::new("Screen")
            .min_size(self.default_size)
            .default_size(self.default_size * 2.0)
            .show(ctx, |ui| {
                egui::Frame::none().show(ui, |ui| {
                    self.paint(ui);
                    let res = ui.interact(
                        ui.min_rect(),
                        egui::Id::new("screen_interact"),
                        egui::Sense::click(),
                    );

                    if res.clicked() {
                        ctx.memory_mut(|m| m.request_focus(egui::Id::new("screen_interact")))
                    }

                    res
                })
            });

        res.and_then(|r| r.inner).map(|r| r.inner)
    }
}
