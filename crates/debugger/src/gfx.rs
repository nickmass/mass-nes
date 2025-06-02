use eframe::{egui::PaintCallbackInfo, egui_glow::Painter, glow};
use glow::HasContext;
use serde::{Deserialize, Serialize};

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use ui::filters::{Filter as FilterTrait, FilterContext, FilterUniforms, Parameter};

use crate::gl::{self, Vertex as _};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    Paletted,
    Ntsc,
    Crt,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Crt
    }
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl gl::Vertex for Vertex {
    const ATTRIBUTES: &[(&'static str, gl::AttrType)] = &[
        ("position", gl::AttrType::Vec2),
        ("tex_coords", gl::AttrType::Vec2),
    ];

    const SIZE: usize = std::mem::size_of::<Vertex>();
}

pub trait SyncFilter: FilterTrait<GlowContext> + Send + Sync + 'static {}

impl<F: FilterTrait<GlowContext> + Send + Sync + 'static> SyncFilter for F {}

pub struct Gfx {
    filter: Option<Box<dyn SyncFilter>>,
    program: Option<gl::Program>,
    vertex_buffer: gl::VertexBuffer<Vertex>,
    frame: Vec<u16>,
    tracy: Tracy,
    back_buffer: GfxBackBuffer,
    selected_filter: Option<Filter>,
    current_filter: Option<Filter>,
}

impl Gfx {
    pub fn new(
        ctx: gl::GlowContext,
        back_buffer: GfxBackBuffer,
        palette: &[u8],
    ) -> Result<Self, String> {
        let ctx = GlowContext(ctx);
        let ver = ctx.version();
        tracing::debug!("OpenGL: ver: {:?} {:?}", ver, Vertex::SIZE);

        let top_right = Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 0.0],
        };
        let top_left = Vertex {
            position: [-1.0, 1.0],
            tex_coords: [0.0, 0.0],
        };
        let bottom_left = Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 1.0],
        };
        let bottom_right = Vertex {
            position: [1.0, -1.0],
            tex_coords: [1.0, 1.0],
        };

        let shape = [top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = gl::VertexBuffer::new(&ctx, gl::Polygon::TriangleFan, &shape)?;

        let tracy = Tracy::new(palette);

        Ok(Self {
            filter: None,
            program: None,
            vertex_buffer,
            back_buffer,
            frame: vec![15; 240 * 256],
            tracy,
            selected_filter: None,
            current_filter: None,
        })
    }

    pub fn filter(&mut self, filter: Filter) {
        self.selected_filter = Some(filter);
    }

    pub fn filter_dimensions(&self) -> (u32, u32) {
        self.filter
            .as_ref()
            .map(|f| f.dimensions())
            .unwrap_or((256, 240))
    }

    pub fn swap(&mut self) {
        self.back_buffer.attempt_swap(&mut self.frame);
    }

    pub fn filter_parameters(&mut self) -> impl Iterator<Item = &mut Parameter<'static>> {
        self.filter.iter_mut().flat_map(|f| f.parameters_mut())
    }

    pub fn render(&mut self, painter: &Painter, paint_info: PaintCallbackInfo) {
        let ctx = GlowContext(painter.gl().clone());
        self.swap();

        if let Some(selected_filter) = self.selected_filter {
            if Some(selected_filter) != self.current_filter {
                let ntsc_setup = ui::filters::NesNtscSetup::composite();

                let filter: Box<dyn SyncFilter> = match selected_filter {
                    Filter::Paletted => Box::new(ui::filters::PalettedFilter::new(
                        ntsc_setup.generate_palette(),
                    )),
                    Filter::Ntsc => Box::new(ui::filters::NtscFilter::new(&ntsc_setup)),
                    Filter::Crt => Box::new(ui::filters::CrtFilter::new(&ntsc_setup)),
                };

                match gl::Program::new(&ctx, filter.vertex_shader(), filter.fragment_shader()) {
                    Ok(new_program) => {
                        let old_program = self.program.take();
                        if let Some(old_program) = old_program {
                            old_program.delete(&ctx);
                        }

                        self.program = Some(new_program);
                        self.filter = Some(filter);
                        self.current_filter = self.selected_filter;
                    }
                    Err(e) => {
                        tracing::error!("unable to compile filter: {e}");
                        self.selected_filter = None;
                    }
                }
            }
        }

        let Some((filter, program)) = self.filter.as_mut().zip(self.program.as_mut()) else {
            return;
        };

        let view = paint_info.viewport_in_pixels();
        let size = (view.width_px as f64, view.height_px as f64);
        let uniforms = filter.process(&ctx, size, &self.frame);
        program.draw(&ctx, &self.vertex_buffer, None, &uniforms);
        self.tracy.frame(&self.frame);
    }
}

#[derive(Clone)]
pub struct GfxBackBuffer {
    repaint: Repainter,
    updated: Arc<AtomicBool>,
    frame: Arc<Mutex<Vec<u16>>>,
}

impl GfxBackBuffer {
    pub fn new(repaint: Repainter) -> Self {
        let frame = Arc::new(Mutex::new(vec![0; 256 * 240]));
        Self {
            repaint,
            frame,
            updated: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update<F: FnOnce(&mut [u16])>(&mut self, func: F) {
        let mut frame = self.frame.lock().unwrap();
        func(&mut frame);
        self.updated.store(true, Ordering::Relaxed);
        self.repaint.request();
    }

    pub fn attempt_swap(&self, other: &mut Vec<u16>) {
        if self.updated.load(Ordering::Relaxed) {
            let Some(mut frame) = self.frame.try_lock().ok() else {
                return;
            };
            std::mem::swap(&mut *frame, other);
            self.updated.store(false, Ordering::Relaxed);
        }
    }
}

pub struct GlowContext(gl::GlowContext);

impl std::ops::Deref for GlowContext {
    type Target = gl::GlowContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for GlowContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FilterContext for GlowContext {
    type Uniforms = gl::Uniforms;

    type Texture = gl::Texture;

    fn create_uniforms(&self) -> Self::Uniforms {
        gl::Uniforms::new(self)
    }

    fn create_texture(&self, params: ui::filters::TextureParams) -> Self::Texture {
        let format = match params.format {
            ui::filters::TextureFormat::RGBA => gl::PixelFormat::RGBA,
            ui::filters::TextureFormat::RGB => gl::PixelFormat::RGB,
            ui::filters::TextureFormat::U16 => gl::PixelFormat::U16,
        };

        let filter = match params.filter {
            ui::filters::TextureFilter::Nearest => gl::TextureFilter::Nearest,
            ui::filters::TextureFilter::Linear => gl::TextureFilter::Linear,
        };

        gl::Texture::new(
            &self,
            format,
            params.width as u16,
            params.height as u16,
            params.pixels,
        )
        .unwrap()
        .with_mag_filter(&self, filter)
        .with_min_filter(&self, filter)
    }
}

impl FilterUniforms<GlowContext> for gl::Uniforms {
    fn add_f32(&mut self, name: &'static str, value: f32) {
        self.add(name, value);
    }

    fn add_vec2(&mut self, name: &'static str, value: (f32, f32)) {
        self.add(name, value);
    }

    fn add_vec4(&mut self, name: &'static str, value: (f32, f32, f32, f32)) {
        self.add(name, value);
    }

    fn add_texture(&mut self, name: &'static str, value: gl::Texture) {
        self.add(name, value);
    }
}

pub use platform::Repainter;
use platform::Tracy;

#[cfg(not(target_arch = "wasm32"))]
use desktop as platform;
#[cfg(target_arch = "wasm32")]
use web as platform;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use eframe::egui;
    use tracy_ext::TracyExt;

    #[derive(Clone)]
    pub struct Repainter {
        ctx: egui::Context,
    }

    impl Repainter {
        pub fn new(ctx: egui::Context) -> Self {
            Self { ctx }
        }

        pub fn request(&mut self) {
            self.ctx.request_repaint();
        }
    }

    pub struct Tracy {
        palette: Box<[u8]>,
        frame_image: Vec<u32>,
    }

    impl Tracy {
        pub fn new(palette: &[u8]) -> Self {
            let frame_image = vec![0; 120 * 128];

            Self {
                palette: palette.into(),
                frame_image,
            }
        }

        pub fn frame(&mut self, screen: &[u16]) {
            if let Some(client) = tracy_client::Client::running() {
                let pixel = |x: usize, y: usize| {
                    let s = screen[y * 256 + x] as usize;
                    let r = self.palette[s * 3 + 2] as u32;
                    let g = self.palette[s * 3 + 1] as u32;
                    let b = self.palette[s * 3 + 0] as u32;

                    [r, g, b]
                };

                for row in 0..120 {
                    for col in 0..128 {
                        let [r0, g0, b0] = pixel(col * 2 + 0, row * 2 + 0);
                        let [r1, g1, b1] = pixel(col * 2 + 1, row * 2 + 0);
                        let [r2, g2, b2] = pixel(col * 2 + 0, row * 2 + 1);
                        let [r3, g3, b3] = pixel(col * 2 + 1, row * 2 + 1);

                        let r = (r0 + r1 + r2 + r3) / 4;
                        let g = (g0 + g1 + g2 + g3) / 4;
                        let b = (b0 + b1 + b2 + b3) / 4;

                        let p = r << 16 | g << 8 | b;

                        self.frame_image[row * 128 + col] = p;
                    }
                }

                client.emit_frame_image(
                    bytemuck::cast_slice(&self.frame_image),
                    128,
                    120,
                    0,
                    false,
                );

                client.frame_mark();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::egui;
    use futures::StreamExt;
    use futures::channel::mpsc::{Sender, channel};

    #[derive(Clone)]
    pub struct Repainter {
        tx: Sender<()>,
    }

    impl Repainter {
        pub fn new(ctx: egui::Context) -> Self {
            let (tx, mut rx) = channel(2);
            let paint = async move {
                loop {
                    rx.next().await;
                    ctx.request_repaint();
                }
            };

            wasm_bindgen_futures::spawn_local(paint);

            Self { tx }
        }

        pub fn request(&mut self) {
            let _ = self.tx.try_send(());
        }
    }

    pub struct Tracy;

    impl Tracy {
        pub fn new(_palette: &[u8]) -> Self {
            Self
        }

        pub fn frame(&mut self, _screen: &[u16]) {}
    }
}
