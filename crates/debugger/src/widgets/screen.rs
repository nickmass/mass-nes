use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};

use crate::{
    egui, egui_glow,
    gfx::{Filter, Gfx},
};
use egui::{Vec2, Widget};
use serde::{Deserialize, Serialize};
use ui::filters::NesNtscSetup;

use super::{Message, PopupMessage};

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
    popup: PopupMessage,
}

impl NesScreen {
    pub fn new(gfx: Gfx) -> Self {
        let gfx = Arc::new(Mutex::new(gfx));
        let (width, height) = gfx.lock().unwrap().filter_dimensions();
        let size = Arc::new(Size::new(width, height));
        let popup = PopupMessage::new();

        Self { gfx, size, popup }
    }

    fn paint(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
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

        if self.popup.has_message() {
            egui::Area::new(egui::Id::new("popup_text"))
                .fixed_pos(rect.left_top())
                .default_size(rect.size())
                .fade_in(true)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .outer_margin(5.0)
                        .show(ui, |ui| {
                            self.popup.show(ui);
                        });
                });
        }
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
        let res = egui::Frame::new().show(ui, |ui| {
            self.paint(ctx, ui);

            let res = ui.interact(ui.min_rect(), SCREEN_INTERACT.into(), egui::Sense::click());

            if res.clicked() {
                self.focus(ctx);
            }

            ui.memory_mut(|m| m.set_focus_lock_filter(SCREEN_INTERACT.into(), focus_filter));
            res
        });

        res.inner
    }

    pub fn set_message(&mut self, message: Message) {
        self.popup.set_message(message);
    }

    pub fn has_message(&self) -> bool {
        self.popup.has_message()
    }

    pub fn configure_filter(&self, ctx: &egui::Context, ntsc_config: &mut NtscConfig) {
        let Ok(mut gfx) = self.gfx.try_lock() else {
            return;
        };

        egui::Window::new("Filter").show(ctx, |ui| {
            let old_preset = ntsc_config.preset;
            egui::ComboBox::from_label("Preset")
                .selected_text(ntsc_config.preset.to_string())
                .show_ui(ui, |ui| {
                    for &option in NtscPreset::all() {
                        ui.selectable_value(&mut ntsc_config.preset, option, option.to_string());
                    }
                });

            if old_preset != ntsc_config.preset {
                *ntsc_config = ntsc_config.preset.default_config();
            }

            egui::Slider::new(&mut ntsc_config.hue, -1.0..=1.0)
                .text("Hue")
                .ui(ui);
            egui::Slider::new(&mut ntsc_config.saturation, -1.0..=1.0)
                .text("Saturation")
                .ui(ui);
            egui::Slider::new(&mut ntsc_config.contrast, -1.0..=1.0)
                .text("Contrast")
                .ui(ui);
            egui::Slider::new(&mut ntsc_config.brightness, -1.0..=1.0)
                .text("Brightness")
                .ui(ui);
            egui::Slider::new(&mut ntsc_config.sharpness, -1.0..=1.0)
                .text("Sharpness")
                .ui(ui);
            ui.checkbox(&mut ntsc_config.merge_fields, "Merge Fields");

            gfx.ntsc_config(ntsc_config.clone());

            let mut first = true;
            for param in gfx.filter_parameters() {
                if first {
                    ui.separator();
                    first = false;
                }
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        param.value = param.default;
                    };
                    egui::Slider::new(&mut param.value, param.min..=param.max)
                        .text(&param.description)
                        .step_by(param.step as f64)
                        .ui(ui);
                });
            }
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NtscConfig {
    preset: NtscPreset,
    hue: f64,
    saturation: f64,
    contrast: f64,
    brightness: f64,
    sharpness: f64,
    merge_fields: bool,
}

impl Default for NtscConfig {
    fn default() -> Self {
        NtscPreset::default().default_config()
    }
}

impl NtscConfig {
    pub fn setup(&self) -> NesNtscSetup {
        let mut setup = self.preset.setup();
        setup.hue = self.hue.min(1.0).max(-1.0);
        setup.saturation = self.saturation.min(1.0).max(-1.0);
        setup.contrast = self.contrast.min(1.0).max(-1.0);
        setup.brightness = self.brightness.min(1.0).max(-1.0);
        setup.sharpness = self.sharpness.min(1.0).max(-1.0);
        setup.merge_fields = self.merge_fields;

        setup
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NtscPreset {
    Monochrome,
    Composite,
    Svideo,
    Rgb,
}

impl Default for NtscPreset {
    fn default() -> Self {
        Self::Composite
    }
}

impl std::fmt::Display for NtscPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NtscPreset::Monochrome => "Monochrome",
            NtscPreset::Composite => "Composite",
            NtscPreset::Svideo => "S-Video",
            NtscPreset::Rgb => "RGB",
        };

        s.fmt(f)
    }
}

impl NtscPreset {
    fn all() -> &'static [Self] {
        &[
            NtscPreset::Monochrome,
            NtscPreset::Composite,
            NtscPreset::Svideo,
            NtscPreset::Rgb,
        ]
    }

    pub fn setup(&self) -> NesNtscSetup {
        match self {
            NtscPreset::Monochrome => NesNtscSetup::monochrome(),
            NtscPreset::Composite => NesNtscSetup::composite(),
            NtscPreset::Svideo => NesNtscSetup::svideo(),
            NtscPreset::Rgb => NesNtscSetup::rgb(),
        }
    }

    fn default_config(&self) -> NtscConfig {
        // Worried about unmerged fields flickering too much with the browsers limited perf.
        let merge_fields = cfg!(target_arch = "wasm32");

        match self {
            NtscPreset::Monochrome => NtscConfig {
                preset: *self,
                hue: 0.0,
                saturation: -1.0,
                contrast: 0.0,
                brightness: 0.0,
                sharpness: 0.2,
                merge_fields,
            },
            NtscPreset::Composite => NtscConfig {
                preset: *self,
                hue: 0.0,
                saturation: 0.0,
                contrast: 0.0,
                brightness: 0.0,
                sharpness: 0.0,
                merge_fields,
            },
            NtscPreset::Svideo => NtscConfig {
                preset: *self,
                hue: 0.0,
                saturation: 0.0,
                contrast: 0.0,
                brightness: 0.0,
                sharpness: 0.2,
                merge_fields,
            },
            NtscPreset::Rgb => NtscConfig {
                preset: *self,
                hue: 0.0,
                saturation: 0.0,
                contrast: 0.0,
                brightness: 0.0,
                sharpness: 0.2,
                merge_fields: true,
            },
        }
    }
}
