use crate::egui;
use egui::{Vec2, Widget};
use tracing::instrument;

use crate::debug_state::{DebugUiState, Nametable, PpuView};

pub struct NametableViewer {
    texture: Option<egui::TextureHandle>,
    age: u64,
    pixel_buf: Vec<u8>,
}

impl NametableViewer {
    pub fn new() -> Self {
        Self {
            texture: None,
            age: 0,
            pixel_buf: vec![0; 4 * 32 * 30 * 8 * 8 * 3],
        }
    }

    fn render_tile<I: Iterator<Item = (u8, u8, u8)>>(
        &mut self,
        nt: usize,
        tile_row: u16,
        tile_col: u16,
        pixels: I,
    ) {
        let tile_row = tile_row as usize;
        let tile_col = tile_col as usize;
        let stride = 2 * 32 * 8;

        let nt_offset = match nt {
            0 => 0,
            1 => 32 * 8,
            2 => 2 * 32 * 30 * 8 * 8,
            3 => (2 * 32 * 30 * 8 * 8) + (32 * 8),
            _ => unreachable!(),
        };

        let pixel_base = nt_offset + (tile_row * 8 * stride) + (tile_col * 8);

        for (idx, (r, g, b)) in pixels.enumerate() {
            let row = idx >> 3;
            let col = idx & 7;
            let pixel_idx = pixel_base + (row as usize * stride) + col;
            self.pixel_buf[pixel_idx * 3 + 0] = r;
            self.pixel_buf[pixel_idx * 3 + 1] = g;
            self.pixel_buf[pixel_idx * 3 + 2] = b;
        }
    }

    #[instrument(skip_all)]
    fn render_nts(&mut self, ppu: PpuView, now: u64, ctx: &egui::Context) {
        for nt_idx in 0..4 {
            let nt = match nt_idx {
                0 => Nametable::One,
                1 => Nametable::Two,
                2 => Nametable::Three,
                3 => Nametable::Four,
                _ => unreachable!(),
            };
            for row in 0..30 {
                for col in 0..32 {
                    let tile_idx = row * 32 + col;
                    let pixels = ppu.nt_tile(nt, tile_idx);
                    self.render_tile(nt_idx, row, col, pixels);
                }
            }
        }

        let image = egui::ColorImage::from_rgb([2 * 32 * 8, 2 * 30 * 8], &self.pixel_buf);

        self.texture = Some(ctx.load_texture("nametables", image, egui::TextureOptions::NEAREST));
        self.age = now;
    }

    fn is_expired(&self, now: u64, debug_interval: u64) -> bool {
        if let Some(_) = &self.texture {
            now - self.age >= debug_interval
        } else {
            true
        }
    }

    pub fn show(&mut self, debug: &DebugUiState, debug_interval: u64, ctx: &egui::Context) {
        let now = debug.now();
        let ppu = debug.ppu();
        if self.is_expired(now, debug_interval) {
            self.render_nts(ppu, now, ctx);
        }
        egui::Window::new("Nametables").show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                egui::Image::new(tex)
                    .fit_to_exact_size(Vec2::new(2.0 * 32.0 * 8.0, 2.0 * 30.0 * 8.0) * 2.0)
                    .ui(ui);
            }
        });
    }
}
