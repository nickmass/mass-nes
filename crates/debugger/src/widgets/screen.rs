use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{
    egui, egui_glow,
    gfx::{Filter, Gfx},
};
use egui::Vec2;

const SCREEN_INTERACT: &'static str = "screen_interact";

struct Size {
    width: AtomicU32,
    height: AtomicU32,
}

impl Size {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
        }
    }

    fn set(&self, width: u32, height: u32) {
        self.width.store(width, Ordering::Relaxed);
        self.height.store(height, Ordering::Relaxed);
    }

    fn as_vec(&self) -> Vec2 {
        let width = self.width.load(Ordering::Relaxed);
        let height = self.height.load(Ordering::Relaxed);
        Vec2::new(width as f32, height as f32)
    }
}

pub struct NesScreen {
    size: Arc<Size>,
    gfx: Arc<Mutex<Gfx>>,
}

impl NesScreen {
    pub fn new(gfx: Gfx) -> Self {
        let gfx = Arc::new(Mutex::new(gfx));
        let (width, height) = gfx.lock().unwrap().filter_dimensions();
        let size = Arc::new(Size::new(width, height));

        Self { gfx, size }
    }

    fn paint(&self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let size = self.size.as_vec();
        let ratio = avail / size;

        let size = if ratio.x > ratio.y {
            size * ratio.y
        } else {
            size * ratio.x
        };

        let (rect, _res) = ui.allocate_exact_size(size, egui::Sense::focusable_noninteractive());

        let gfx = self.gfx.clone();
        let size = self.size.clone();
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |paint_info, painter| {
                let mut gfx = gfx.lock().unwrap();
                gfx.render(painter, paint_info);
                let (width, height) = gfx.filter_dimensions();
                size.set(width, height);
            })),
        };

        ui.painter().add(callback);
    }

    pub fn filter(&mut self, filter: Filter) {
        let mut gfx = self.gfx.lock().unwrap();
        gfx.filter(filter);
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(SCREEN_INTERACT.into()))
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<egui::Response> {
        let size = self.size.as_vec();
        let res = egui::Window::new("Screen")
            .min_size(size)
            .default_size(size * 2.0)
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
