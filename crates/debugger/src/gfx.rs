use eframe::{egui, egui::PaintCallbackInfo, glow};
use glow::HasContext;
use tracy_ext::TracyExt;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use ui::filters::{Filter, FilterContext, FilterUniforms};

use crate::gl::{self, Vertex as _};

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

pub struct Gfx<T> {
    filter: T,
    ctx: GlowContext,
    program: gl::Program,
    vertex_buffer: gl::VertexBuffer<Vertex>,
    size: (f64, f64),
    frame: Vec<u16>,
    tracy: Tracy,
    back_buffer: GfxBackBuffer,
}

impl<T: Filter> Gfx<T> {
    pub fn new(
        ctx: gl::GlowContext,
        back_buffer: GfxBackBuffer,
        palette: &[u8],
        filter: T,
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

        let program = gl::Program::new(&ctx, filter.vertex_shader(), filter.fragment_shader())?;

        let size = filter.dimensions();
        let size = (size.0 as f64, size.1 as f64);

        let tracy = Tracy::new(palette);

        Ok(Self {
            filter,
            ctx,
            program,
            vertex_buffer,
            size,
            back_buffer,
            frame: vec![15; 240 * 256],
            tracy,
        })
    }

    pub fn filter_dimensions(&self) -> (u32, u32) {
        self.filter.dimensions()
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        let size = (size.0 as f64, size.1 as f64);
        self.size = size;
    }

    pub fn swap(&mut self) {
        self.back_buffer.attempt_swap(&mut self.frame);
    }

    pub fn render(&mut self, paint_info: PaintCallbackInfo) {
        self.swap();
        let view = paint_info.viewport_in_pixels();
        self.resize((view.width_px as u32, view.height_px as u32));
        let uniforms = self.filter.process(&self.ctx, self.size, &self.frame);
        self.program.draw(&self.vertex_buffer, &uniforms);
        self.tracy.frame(&self.frame);
    }
}

#[derive(Clone)]
pub struct GfxBackBuffer {
    ctx: egui::Context,
    updated: Arc<AtomicBool>,
    frame: Arc<Mutex<Vec<u16>>>,
}

impl GfxBackBuffer {
    pub fn new(ctx: egui::Context) -> Self {
        let frame = Arc::new(Mutex::new(vec![0; 256 * 240]));
        Self {
            ctx,
            frame,
            updated: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update<F: FnOnce(&mut [u16])>(&mut self, func: F) {
        {
            let mut frame = self.frame.lock().unwrap();
            func(&mut frame);
            self.updated.store(true, Ordering::Relaxed);
            self.ctx.request_repaint();
        }
    }

    pub fn attempt_swap(&self, other: &mut Vec<u16>) {
        if self.updated.load(Ordering::Relaxed) {
            let mut frame = self.frame.lock().unwrap();
            std::mem::swap(&mut *frame, other);
            self.updated.store(false, Ordering::Relaxed);
        }
    }
}

struct Tracy {
    palette: Box<[u8]>,
    frame_image: Vec<u32>,
}

impl Tracy {
    fn new(palette: &[u8]) -> Self {
        let frame_image = vec![0; 120 * 128];

        Self {
            palette: palette.into(),
            frame_image,
        }
    }

    #[tracing::instrument(skip_all)]
    fn frame(&mut self, screen: &[u16]) {
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

                    let r = ((r0 + r1 + r2 + r3) as f32 / 4.0) as u32;
                    let g = ((g0 + g1 + g2 + g3) as f32 / 4.0) as u32;
                    let b = ((b0 + b1 + b2 + b3) as f32 / 4.0) as u32;

                    let p = r << 16 | g << 8 | b;

                    self.frame_image[row * 128 + col] = p;
                }
            }

            client.emit_frame_image(bytemuck::cast_slice(&self.frame_image), 128, 120, 0, false);

            client.frame_mark();
        }
    }
}

struct GlowContext(gl::GlowContext);

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
        gl::Uniforms::new()
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
        .with_mag_filter(filter)
        .with_min_filter(filter)
    }
}

impl FilterUniforms<GlowContext> for gl::Uniforms {
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32)) {
        self.add(name, value);
    }

    fn add_texture(&mut self, name: &'static str, value: gl::Texture) {
        self.add(name, value);
    }
}
