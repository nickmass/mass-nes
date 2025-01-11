use std::sync::{Arc, Mutex};

use crate::{
    egui, egui_glow,
    gfx::{Filter, Gfx},
};
use egui::Vec2;

const SCREEN_INTERACT: &'static str = "screen_interact";

pub struct NesScreen {
    default_size: Vec2,
    gfx: Arc<Mutex<Gfx>>,
}

impl NesScreen {
    pub fn new(gfx: Gfx) -> Self {
        let gfx = Arc::new(Mutex::new(gfx));
        let (width, height) = gfx.lock().unwrap().filter_dimensions();
        let default_size = Vec2::new(width as f32, height as f32);

        Self { gfx, default_size }
    }

    fn paint(&self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let ratio = avail / self.default_size;

        let size = if ratio.x > ratio.y {
            self.default_size * ratio.y
        } else {
            self.default_size * ratio.x
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

    pub fn filter(&mut self, filter: Filter) {
        let mut gfx = self.gfx.lock().unwrap();
        gfx.filter(filter);
        let (width, height) = gfx.filter_dimensions();
        self.default_size = Vec2::new(width as f32, height as f32);
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(SCREEN_INTERACT.into()))
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<egui::Response> {
        let res = egui::Window::new("Screen")
            .min_size(self.default_size)
            .default_size(self.default_size * 2.0)
            .show(ctx, |ui| self.fill(ctx, ui));

        res.and_then(|r| r.inner)
    }

    pub fn fill(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> egui::Response {
        let focus_filter = egui::EventFilter {
            tab: true,
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: false,
        };
        let res = egui::Frame::none().show(ui, |ui| {
            self.paint(ui);

            let res = ui.interact(ui.min_rect(), SCREEN_INTERACT.into(), egui::Sense::click());

            if res.clicked() {
                self.focus(ctx);
            }

            ui.memory_mut(|m| m.set_focus_lock_filter(SCREEN_INTERACT.into(), focus_filter));

            res
        });

        res.inner
    }
}
