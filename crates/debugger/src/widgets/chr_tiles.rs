use crate::egui;
use egui::{Vec2, Widget};

use super::PaletteViewer;
use crate::debug_state::{ChrTable, DebugUiState, PpuView};

struct ChrImage {
    age: u64,
    palette: u8,
    texture: egui::TextureHandle,
}

pub struct ChrTiles {
    lo_chr: Option<ChrImage>,
    hi_chr: Option<ChrImage>,
    pixel_buf: Vec<u8>,
}

impl ChrTiles {
    pub fn new() -> Self {
        Self {
            lo_chr: None,
            hi_chr: None,
            pixel_buf: vec![0; 3 * 256 * 64],
        }
    }

    fn render_tile(&mut self, ppu: &PpuView, chr_table: ChrTable, palette: u8, tile_idx: u8) {
        let pixel_base = (tile_idx as usize % 16 * 8) + (tile_idx as usize / 16 * (64 * 16));
        for (idx, (r, g, b)) in ppu.chr_tile(palette, chr_table, tile_idx).enumerate() {
            let row = idx >> 3;
            let col = idx & 7;
            let pixel_idx = pixel_base + (row as usize * 16 * 8) + col;
            self.pixel_buf[pixel_idx * 3 + 0] = r;
            self.pixel_buf[pixel_idx * 3 + 1] = g;
            self.pixel_buf[pixel_idx * 3 + 2] = b;
        }
    }

    fn is_expired(&self, table: ChrTable, palette: u8, now: u64, debug_interval: u64) -> bool {
        let image = match table {
            ChrTable::Low => self.lo_chr.as_ref(),
            ChrTable::High => self.hi_chr.as_ref(),
        };

        if let Some(image) = image {
            now - image.age >= debug_interval || image.palette != palette
        } else {
            true
        }
    }

    fn update_image(
        &mut self,
        ppu: &PpuView,
        chr_table: ChrTable,
        palette: u8,
        now: u64,
        ctx: &egui::Context,
    ) -> ChrImage {
        for tile in 0..=255 {
            self.render_tile(ppu, chr_table, palette, tile);
        }

        let image = egui::ColorImage::from_rgb([16 * 8, 16 * 8], &self.pixel_buf);
        let name = match chr_table {
            ChrTable::Low => "lo_chr_table",
            ChrTable::High => "hi_chr_table",
        };

        let texture = ctx.load_texture(name, image, egui::TextureOptions::NEAREST);

        ChrImage {
            age: now,
            palette,
            texture,
        }
    }

    fn update_images(
        &mut self,
        ppu: &PpuView,
        palette: u8,
        now: u64,
        debug_interval: u64,
        ctx: &egui::Context,
    ) {
        if self.is_expired(ChrTable::Low, palette, now, debug_interval) {
            let image = self.update_image(ppu, ChrTable::Low, palette, now, ctx);
            self.lo_chr = Some(image);
        }

        if self.is_expired(ChrTable::High, palette, now, debug_interval) {
            let image = self.update_image(ppu, ChrTable::High, palette, now, ctx);
            self.hi_chr = Some(image);
        }
    }

    pub fn show(
        &mut self,
        selected_palette: &mut u8,
        debug: &DebugUiState,
        debug_interval: u64,
        ctx: &egui::Context,
    ) {
        let ppu = debug.ppu();
        let now = debug.now();
        self.update_images(&ppu, *selected_palette, now, debug_interval, ctx);

        egui::Window::new("CHR Tiles")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(img) = &self.lo_chr {
                        egui::Image::new(&img.texture)
                            .fit_to_exact_size(Vec2::splat(256.0))
                            .ui(ui);
                    }

                    if let Some(img) = &self.hi_chr {
                        egui::Image::new(&img.texture)
                            .fit_to_exact_size(Vec2::splat(256.0))
                            .ui(ui);
                    }
                });
                PaletteViewer::new(debug.ppu()).ui(selected_palette, ui);
            });
    }
}
