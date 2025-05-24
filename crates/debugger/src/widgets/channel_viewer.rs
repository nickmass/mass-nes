use nes::ChannelSamples;

use crate::debug_state::DebugUiState;
use crate::egui;

pub struct ChannelViewer {
    images: [ChannelImage; 6],
    prev: [Option<f32>; 6],
    age: u64,
}

impl ChannelViewer {
    pub fn new() -> Self {
        Self {
            images: std::array::from_fn(|_| ChannelImage::new()),
            prev: [None; 6],
            age: 0,
        }
    }

    fn update(&mut self, ctx: &egui::Context, channels: &[ChannelSamples]) {
        let chunk = channels.len() as f32 / WIDTH as f32;
        let mut chunk_accum = 0.0;

        let mut channel_iter = channels.iter();

        for x in 0..WIDTH {
            chunk_accum += chunk;
            let h = chunk_accum.floor();
            chunk_accum -= h;

            let mut sum = [0.0; 6];

            for _ in 0..h as u32 {
                if let Some(n) = channel_iter.next() {
                    for (idx, sum) in sum.iter_mut().enumerate() {
                        *sum += Channel::from_idx(idx).value(n);
                    }
                }
            }

            for ((idx, sum), prev) in sum.into_iter().enumerate().zip(self.prev.iter_mut()) {
                let a = sum / h;

                if let Some(b) = prev {
                    self.images[idx].draw_column(&ctx.style().visuals, x, a.min(*b), a.max(*b));
                }

                *prev = Some(a);
            }
        }

        for (idx, channel) in Channel::all().into_iter().enumerate() {
            self.images[idx].update(channel.name(), ctx);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, debug: &DebugUiState, debug_interval: u64) {
        if self.age.abs_diff(debug.now()) >= debug_interval {
            self.update(ctx, debug.channels());
            self.age = debug.now();
        }

        egui::Window::new("Audio Channels")
            .auto_sized()
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for idx in 0..3 {
                        let channel = Channel::from_idx(idx);
                        self.images[idx].show(ui, channel.label());
                    }
                });
                ui.horizontal(|ui| {
                    for idx in 3..6 {
                        let channel = Channel::from_idx(idx);
                        self.images[idx].show(ui, channel.label());
                    }
                });
            });
    }
}

#[derive(Debug, Copy, Clone)]
enum Channel {
    Pulse1,
    Pulse2,
    Triangle,
    Noise,
    Dmc,
    External,
}

impl Channel {
    const fn all() -> [Self; 6] {
        use Channel::*;
        [Pulse1, Pulse2, Triangle, Noise, Dmc, External]
    }

    fn from_idx(idx: usize) -> Self {
        Self::all()[idx]
    }

    fn label(&self) -> &'static str {
        match self {
            Channel::Pulse1 => "Pulse",
            Channel::Pulse2 => "Pulse",
            Channel::Triangle => "Triangle",
            Channel::Noise => "Noise",
            Channel::Dmc => "DMC",
            Channel::External => "External",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Channel::Pulse1 => "pulse_1",
            Channel::Pulse2 => "pulse_2",
            Channel::Triangle => "triangle",
            Channel::Noise => "noise",
            Channel::Dmc => "dmc",
            Channel::External => "external",
        }
    }

    fn value(&self, channel: &ChannelSamples) -> f32 {
        match self {
            Channel::Pulse1 => channel.pulse_1,
            Channel::Pulse2 => channel.pulse_2,
            Channel::Triangle => channel.triangle,
            Channel::Noise => channel.noise,
            Channel::Dmc => channel.dmc,
            Channel::External => channel.external,
        }
    }
}

const WIDTH: usize = 256;
const HEIGHT: usize = 256;

struct ChannelImage {
    pixels: Vec<u8>,
    texture: Option<egui::TextureHandle>,
}

impl ChannelImage {
    fn new() -> Self {
        Self {
            pixels: vec![0x00; (WIDTH * HEIGHT * 3) as usize],
            texture: None,
        }
    }

    fn draw_column(&mut self, visuals: &egui::Visuals, x: usize, min: f32, max: f32) {
        let min = (min * (HEIGHT - 1) as f32) as usize;
        let max = (max * (HEIGHT - 1) as f32) as usize;

        for y in 0..HEIGHT {
            let idx = ((HEIGHT - 1 - y) * WIDTH + x) * 3;

            let color = if y >= min && y <= max {
                visuals.hyperlink_color
            } else {
                visuals.panel_fill
            };

            self.pixels[idx] = color.r();
            self.pixels[idx + 1] = color.g();
            self.pixels[idx + 2] = color.b();
        }
    }

    fn update(&mut self, name: &'static str, ctx: &egui::Context) {
        let image = egui::ColorImage::from_rgb([WIDTH, HEIGHT], &self.pixels);
        let texture = ctx.load_texture(name, image, egui::TextureOptions::NEAREST);
        self.texture = Some(texture);
    }

    fn show(&self, ui: &mut egui::Ui, label: &'static str) {
        if let Some(tex) = self.texture.as_ref() {
            ui.vertical(|ui| {
                ui.label(label);
                ui.image(tex);
            });
        }
    }
}
