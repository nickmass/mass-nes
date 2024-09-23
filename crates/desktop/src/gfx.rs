use glium::glutin::surface::WindowSurface;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d, UnsignedTexture2d};
use glium::uniforms::{UniformValue, Uniforms};
use glium::{implement_vertex, Display, Program, Surface, VertexBuffer};

use std::borrow::Cow;

use ui::filters::{Filter, FilterContext, FilterUniforms, NesNtscSetup, TextureFormat};

use crate::TracyExt;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct Gfx<T> {
    filter: T,
    display: GliumContext,
    indicies: glium::index::NoIndices,
    program: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    size: (f64, f64),
    frame: Option<Vec<u16>>,
    tracy: Tracy,
}

impl<T: Filter> Gfx<T> {
    pub fn new(display: Display<WindowSurface>, filter: T) -> Self {
        let ver = display.get_opengl_version_string();
        let glsl = display.get_supported_glsl_version();
        let vendor = display.get_opengl_vendor_string();
        let renderer = display.get_opengl_renderer_string();
        tracing::debug!(
            "OpenGL: ver: {}, glsl: {:?}, vendor: {}, renderer: {}",
            ver,
            glsl,
            vendor,
            renderer
        );

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

        let vertex_buffer = VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let program = Program::from_source(
            &display,
            &*filter.vertex_shader(),
            &*filter.fragment_shader(),
            None,
        );

        let program = match program {
            Ok(p) => p,
            Err(glium::CompilationError(msg, kind)) => {
                panic!("Shader Compilation Errror '{kind:?}':\n{msg}")
            }
            Err(e) => panic!("{e:?}"),
        };

        let size = filter.dimensions();
        let size = (size.0 as f64, size.1 as f64);

        let tracy = Tracy::new();

        Self {
            filter,
            display: GliumContext(display),
            indicies,
            program,
            vertex_buffer,
            size,
            frame: None,
            tracy,
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.display.resize(size);
        let size = (size.0 as f64, size.1 as f64);
        self.size = size;
    }

    pub fn update_frame(&mut self, frame: Vec<u16>) {
        self.frame = Some(frame);
    }

    pub fn render(&mut self) {
        let Some(screen) = self.frame.as_ref() else {
            return;
        };
        let uniforms = self.filter.process(&self.display, self.size, screen);
        let mut target = self.display.draw();

        let (filter_width, filter_height) = self.filter.dimensions();
        let (filter_width, filter_height) = (filter_width as f64, filter_height as f64);
        let (window_width, window_height) = self.size;
        let (surface_width, surface_height) = target.get_dimensions();
        let (surface_width, surface_height) = (surface_width as f64, surface_height as f64);
        let filter_ratio = filter_width / filter_height;
        let surface_ratio = surface_width / surface_height;

        let (left, bottom, width, height) = if filter_ratio > surface_ratio {
            let target_height = (1.0 / filter_ratio) * window_height;
            let target_height = (target_height / window_height) * surface_height * surface_ratio;
            (
                0,
                ((surface_height - target_height) / 2.0) as u32,
                surface_width as u32,
                target_height as u32,
            )
        } else {
            let target_width = (filter_ratio) * window_width;
            let target_width =
                (target_width / window_width) * surface_width * (1.0 / surface_ratio);
            (
                ((surface_width - target_width) / 2.0) as u32,
                0,
                target_width as u32,
                surface_height as u32,
            )
        };

        let params = glium::DrawParameters {
            viewport: Some(glium::Rect {
                left,
                bottom,
                width,
                height,
            }),
            ..Default::default()
        };

        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &self.vertex_buffer,
                &self.indicies,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
        target.finish().unwrap();
        self.tracy.frame(screen);
    }
}

struct Tracy {
    palette: Box<[u8]>,
    frame_image: Vec<u32>,
}

impl Tracy {
    fn new() -> Self {
        let ntsc = NesNtscSetup::rgb();
        let palette = ntsc.generate_palette().into();
        let frame_image = vec![0; 120 * 128];

        Self {
            palette,
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

struct GliumContext(Display<WindowSurface>);

impl std::ops::Deref for GliumContext {
    type Target = Display<WindowSurface>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for GliumContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FilterContext for GliumContext {
    type Uniforms = UniformCollection<'static>;

    type Texture = Texture;

    fn create_uniforms(&self) -> Self::Uniforms {
        UniformCollection::new()
    }

    fn create_texture(&self, params: ui::filters::TextureParams) -> Self::Texture {
        let filter = match params.filter {
            ui::filters::TextureFilter::Nearest => FilterScaling::Nearest,
            ui::filters::TextureFilter::Linear => FilterScaling::Linear,
        };
        match params.format {
            f @ TextureFormat::RGBA | f @ TextureFormat::RGB => {
                let pixel_format = match f {
                    TextureFormat::RGBA => ClientFormat::U8U8U8U8,
                    TextureFormat::RGB => ClientFormat::U8U8U8,
                    TextureFormat::U16 => ClientFormat::U8U8,
                };

                let img = RawImage2d {
                    data: Cow::Borrowed(params.pixels),
                    width: params.width as u32,
                    height: params.height as u32,
                    format: pixel_format,
                };

                let tex = Texture2d::with_mipmaps(&self.0, img, MipmapsOption::NoMipmap).unwrap();

                Texture::Texture2d(tex, filter)
            }
            TextureFormat::U16 => {
                let img = RawImage2d {
                    data: Cow::Borrowed(params.pixels),
                    width: params.width as u32,
                    height: params.height as u32,
                    format: ClientFormat::U8U8,
                };

                let tex =
                    UnsignedTexture2d::with_mipmaps(&self.0, img, MipmapsOption::NoMipmap).unwrap();

                Texture::UTexture2d(tex, filter)
            }
        }
    }
}

impl FilterUniforms<GliumContext> for UniformCollection<'static> {
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32)) {
        self.add(name, value);
    }

    fn add_texture(&mut self, name: &'static str, value: Texture) {
        match value {
            Texture::Texture2d(tex, scale) => self.add_2d_uniform(name, tex, scale),
            Texture::UTexture2d(tex, scale) => self.add_u2d_uniform(name, tex, scale),
        }
    }
}

enum Texture {
    Texture2d(Texture2d, FilterScaling),
    UTexture2d(UnsignedTexture2d, FilterScaling),
}

pub enum FilterScaling {
    Nearest,
    Linear,
}

enum FilterTexture {
    Texture2d(Texture2d),
    UnsignedTexture2d(UnsignedTexture2d),
}

enum FilterUniform<'a> {
    Sampler(FilterSampler<'a>),
    Simple {
        name: &'a str,
        value: UniformValue<'static>,
    },
}

pub struct FilterSampler<'a> {
    name: &'a str,
    texture: FilterTexture,
    scaling: FilterScaling,
}

pub struct UniformCollection<'a> {
    uniforms: Vec<FilterUniform<'a>>,
}

impl<'a> UniformCollection<'a> {
    pub fn new() -> Self {
        Self {
            uniforms: Vec::new(),
        }
    }

    pub fn add_2d_uniform(&mut self, name: &'a str, tex: Texture2d, scale: FilterScaling) {
        let uni = FilterSampler {
            name,
            texture: FilterTexture::Texture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(FilterUniform::Sampler(uni));
    }

    pub fn add_u2d_uniform(&mut self, name: &'a str, tex: UnsignedTexture2d, scale: FilterScaling) {
        let uni = FilterSampler {
            name,
            texture: FilterTexture::UnsignedTexture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(FilterUniform::Sampler(uni));
    }

    pub fn add<T: ToUniform>(&mut self, name: &'a str, value: T) {
        self.uniforms.push(FilterUniform::Simple {
            name: name.into(),
            value: value.to_uniform(),
        })
    }
}

impl<'a> Uniforms for UniformCollection<'a> {
    fn visit_values<'b, F: FnMut(&str, glium::uniforms::UniformValue<'b>)>(&'b self, mut visit: F) {
        use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
        for uni in self.uniforms.iter() {
            match uni {
                FilterUniform::Simple { name, value } => visit(&*name, value.clone()),
                FilterUniform::Sampler(uni) => {
                    let (mag_scale, min_scale) = match uni.scaling {
                        FilterScaling::Nearest => {
                            (MagnifySamplerFilter::Nearest, MinifySamplerFilter::Nearest)
                        }
                        FilterScaling::Linear => {
                            (MagnifySamplerFilter::Linear, MinifySamplerFilter::Linear)
                        }
                    };

                    let mut sampler = glium::uniforms::SamplerBehavior::default();
                    sampler.magnify_filter = mag_scale;
                    sampler.minify_filter = min_scale;

                    match uni.texture {
                        FilterTexture::Texture2d(ref tex) => {
                            visit(uni.name, UniformValue::Texture2d(tex, Some(sampler)));
                        }
                        FilterTexture::UnsignedTexture2d(ref tex) => {
                            visit(
                                uni.name,
                                UniformValue::UnsignedTexture2d(tex, Some(sampler)),
                            );
                        }
                    }
                }
            }
        }
    }
}

pub trait ToUniform {
    fn to_uniform(self) -> UniformValue<'static>;
}

impl ToUniform for (f32, f32) {
    fn to_uniform(self) -> UniformValue<'static> {
        UniformValue::Vec2([self.0, self.1])
    }
}
