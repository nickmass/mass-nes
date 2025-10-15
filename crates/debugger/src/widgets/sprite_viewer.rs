use crate::egui;
use egui::{Rect, TextureHandle, Vec2, Widget};

use crate::debug_state::{DebugUiState, PpuView, Sprite};

struct SpriteImage {
    idx: u8,
    sprite: Sprite,
    uv: Rect,
}

impl SpriteImage {
    fn ui(&self, texture: &TextureHandle, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.vertical(|ui| {
                ui.label(format!("#{:02}", self.idx));
                ui.label(format!("x: {:03}", self.sprite.x()));
                ui.label(format!("y: {:03}", self.sprite.y()));
                if self.sprite.behind_bg() {
                    ui.label("bg");
                }
            });

            egui::Image::new(texture)
                .uv(self.uv)
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(self.size() * 4.0)
                .ui(ui);
        });
    }

    fn size(&self) -> Vec2 {
        if self.sprite.tall {
            Vec2::new(8.0, 16.0)
        } else {
            Vec2::new(8.0, 8.0)
        }
    }

    fn hidden(&self) -> bool {
        self.sprite.hidden()
    }
}

pub struct SpriteViewer {
    sprites: Vec<SpriteImage>,
    pixel_buf: Vec<u8>,
    texture: Option<TextureHandle>,
    age: u64,
    show_bytes: bool,
}

impl SpriteViewer {
    pub fn new() -> Self {
        Self {
            sprites: Vec::with_capacity(64),
            pixel_buf: vec![0; 8 * 16 * 8 * 8 * 3],
            texture: None,
            age: 0,
            show_bytes: false,
        }
    }

    fn render_sprites(&mut self, ppu: &PpuView, now: u64, ctx: &egui::Context) {
        self.sprites.clear();

        for idx in 0..64 {
            let col = (idx % 8) as f32;
            let row = (idx / 8) as f32;
            let sprite = ppu.sprite(idx);

            let uv = if sprite.tall {
                let width = 1.0 / 8.0;
                let height = 1.0 / 8.0;

                Rect::from_min_size((col * width, row * height).into(), (width, height).into())
            } else {
                let width = 1.0 / 8.0;
                let height = 1.0 / 16.0;

                Rect::from_min_size(
                    (col * width, row * height * 2.0).into(),
                    (width, height).into(),
                )
            };

            let sprite = SpriteImage { idx, sprite, uv };

            self.sprites.push(sprite);
        }

        self.pixel_buf.fill(0);

        let stride = 8 * 8;
        for (idx, sprite) in self.sprites.iter().enumerate() {
            let x = (idx % 8) * 8;
            let y = (idx / 8) * 16;
            let base_off = y * stride + x;

            for (idx, &(r, g, b)) in sprite.sprite.pixels.iter().enumerate() {
                let col = idx % 8;
                let row = idx / 8;

                let pixel_idx = (base_off + (row * stride + col)) * 3;
                self.pixel_buf[pixel_idx + 0] = r;
                self.pixel_buf[pixel_idx + 1] = g;
                self.pixel_buf[pixel_idx + 2] = b;
            }
        }

        let image = egui::ColorImage::from_rgb([8 * 8, 16 * 8], &self.pixel_buf);

        self.texture = Some(ctx.load_texture("sprite_sheet", image, egui::TextureOptions::NEAREST));
        self.age = now;
    }

    fn is_expired(&self, now: u64, debug_interval: u64) -> bool {
        if let Some(_) = &self.texture {
            now - self.age >= debug_interval
        } else {
            true
        }
    }

    pub fn show(
        &mut self,
        show_all_sprites: &mut bool,
        debug: &DebugUiState,
        debug_interval: u64,
        ctx: &egui::Context,
    ) {
        let now = debug.now();
        let ppu = &debug.ppu();

        if self.is_expired(now, debug_interval) {
            self.render_sprites(ppu, now, ctx);
        }

        egui::Window::new("Sprites")
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(tex) = &self.texture {
                    ui.horizontal(|ui| {
                        ui.checkbox(show_all_sprites, "Show hidden sprites");
                        ui.checkbox(&mut self.show_bytes, "Show bytes");
                    });
                    ui.separator();
                    let mut any_sprites = false;

                    if self.show_bytes {
                        egui::Grid::new("sprite_bytes_grid")
                            .num_columns(17)
                            .show(ui, |ui| {
                                for i in 0..16 {
                                    let base_addr = i * 16;
                                    ui.label(format!("0x{base_addr:02X}:"));
                                    for j in 0..16 {
                                        let val = ppu.sprite_ram()[base_addr + j];
                                        ui.label(format!("0x{val:02X}"));
                                    }

                                    ui.end_row();
                                }
                            });
                    } else {
                        egui::Grid::new("sprite_viewer_grid")
                            .num_columns(8)
                            .min_row_height(16.0 * 4.0)
                            .show(ui, |ui| {
                                for (idx, sprite) in self
                                    .sprites
                                    .iter()
                                    .filter(|s| !s.hidden() || *show_all_sprites)
                                    .enumerate()
                                {
                                    sprite.ui(tex, ui);
                                    any_sprites = true;
                                    if (idx + 1) % 8 == 0 {
                                        ui.end_row();
                                    }
                                }
                                if !any_sprites {
                                    ui.label("No visible sprites");
                                }
                                ui.end_row();
                            });
                    }
                }
            });
    }
}
