use glium;
pub use glium::glutin::VirtualKeyCode as Key;
use glium::texture;
use glium::texture::integral_texture2d::IntegralTexture2d;
use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};
use glium::Surface;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

pub enum FilterScaling {
    Nearest,
    Linear,
}

enum FilterTexture {
    Texture2d(Texture2d),
    IntegralTexture2d(IntegralTexture2d),
}

pub struct FilterUniform {
    name: String,
    texture: FilterTexture,
    scaling: FilterScaling,
}

pub struct FilterUniforms {
    uniforms: Vec<FilterUniform>,
}

impl FilterUniforms {
    pub fn new() -> FilterUniforms {
        FilterUniforms {
            uniforms: Vec::new(),
        }
    }

    pub fn add_2d_uniform(&mut self, name: String, tex: Texture2d, scale: FilterScaling) {
        let uni = FilterUniform {
            name: name,
            texture: FilterTexture::Texture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(uni);
    }

    pub fn add_i2d_uniform(&mut self, name: String, tex: IntegralTexture2d, scale: FilterScaling) {
        let uni = FilterUniform {
            name: name,
            texture: FilterTexture::IntegralTexture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(uni);
    }
}

impl glium::uniforms::Uniforms for FilterUniforms {
    fn visit_values<'b, F: FnMut(&str, glium::uniforms::UniformValue<'b>)>(&'b self, mut visit: F) {
        use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, UniformValue};
        for uni in self.uniforms.iter() {
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
                    visit(&*uni.name, UniformValue::Texture2d(&tex, Some(sampler)));
                }
                FilterTexture::IntegralTexture2d(ref tex) => {
                    visit(
                        &*uni.name,
                        UniformValue::IntegralTexture2d(&tex, Some(sampler)),
                    );
                }
            }
        }
    }
}

pub trait Filter {
    fn get_dimensions(&self) -> (u32, u32);
    fn get_fragment_shader(&self) -> String;
    fn get_vertex_shader(&self) -> String;
    fn process(
        &self,
        display: &glium::Display,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> FilterUniforms;
}

pub struct PalettedFilter {
    palette: [u8; 1536],
}

impl PalettedFilter {
    pub fn new(pal: [u8; 1536]) -> PalettedFilter {
        PalettedFilter { palette: pal }
    }
}

impl Filter for PalettedFilter {
    fn get_dimensions(&self) -> (u32, u32) {
        (256 * 3, 240 * 3)
    }

    fn get_fragment_shader(&self) -> String {
        r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            uniform isampler2D tex;
            uniform sampler2D palette;

            void main() {
                ivec4 index = texture(tex, v_tex_coords);
                color = texelFetch(palette, ivec2(index.x % 64, index.x / 64), 0);
            }
        "#
        .to_string()
    }

    fn get_vertex_shader(&self) -> String {
        r#"
            #version 140

            in vec2 position;
            in vec2 tex_coords;

            out vec2 v_tex_coords;

            void main() {
                v_tex_coords = tex_coords;
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "#
        .to_string()
    }

    fn process(
        &self,
        display: &glium::Display,
        _render_size: (f64, f64),
        screen: &[u16],
    ) -> FilterUniforms {
        let mut unis = FilterUniforms::new();
        let img = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(screen),
            width: 256,
            height: 240,
            format: ClientFormat::U16,
        };

        let tex = IntegralTexture2d::with_mipmaps(&*display, img, texture::MipmapsOption::NoMipmap)
            .unwrap();
        unis.add_i2d_uniform("tex".to_string(), tex, FilterScaling::Nearest);

        let palette = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(&self.palette[..]),
            width: 64,
            height: 8,
            format: ClientFormat::U8U8U8,
        };

        let pal_tex =
            Texture2d::with_mipmaps(&*display, palette, texture::MipmapsOption::NoMipmap).unwrap();
        unis.add_2d_uniform("palette".to_string(), pal_tex, FilterScaling::Nearest);
        unis
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct GliumRenderer<T: Filter> {
    filter: T,
    events_loop: RefCell<glium::glutin::EventsLoop>,
    display: RefCell<glium::Display>,
    indicies: glium::index::NoIndices,
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    closed: Cell<bool>,
    input: RefCell<HashMap<Key, bool>>,
    size: Cell<(f64, f64)>,
}

impl<T: Filter> GliumRenderer<T> {
    pub fn new(filter: T) -> GliumRenderer<T> {
        let dims = filter.get_dimensions();
        let events_loop = glium::glutin::EventsLoop::new();
        let window = glium::glutin::WindowBuilder::new()
            .with_dimensions(glium::glutin::dpi::LogicalSize::from_physical(
                (dims.0, dims.1),
                2.0,
            ))
            .with_title("Mass NES");

        let context = glium::glutin::ContextBuilder::new();

        let display = glium::Display::new(window, context, &events_loop)
            .expect("Could not initialize display");

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

        let shape = vec![top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let program = glium::Program::from_source(
            &display,
            &*filter.get_vertex_shader(),
            &*filter.get_fragment_shader(),
            None,
        )
        .unwrap();

        GliumRenderer {
            filter: filter,
            events_loop: RefCell::new(events_loop),
            display: RefCell::new(display),
            indicies: indicies,
            program: program,
            vertex_buffer: vertex_buffer,
            closed: Cell::new(false),
            input: RefCell::new(HashMap::new()),
            size: Cell::new((dims.0 as f64, dims.1 as f64)),
        }
    }

    fn process_events(&self) {
        self.events_loop.borrow_mut().poll_events(|event| {
            use glium::glutin::Event::*;
            match event {
                WindowEvent { event, .. } => {
                    use glium::glutin::WindowEvent::*;
                    match event {
                        Resized(new_size) => {
                            self.size.set(new_size.into());
                        }
                        CloseRequested => {
                            self.closed.set(true);
                        }
                        _ => (),
                    }
                }
                DeviceEvent { event, .. } => {
                    use glium::glutin::DeviceEvent::*;
                    match event {
                        Key(key) => {
                            let pressed = key.state == glium::glutin::ElementState::Pressed;
                            if let Some(key) = key.virtual_keycode {
                                self.input.borrow_mut().insert(key, pressed);
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        });
    }

    pub fn render(&self, screen: &[u16]) {
        let uniforms = self
            .filter
            .process(&*self.display.borrow(), self.size.get(), screen);
        let mut target = self.display.borrow_mut().draw();

        let (filter_width, filter_height) = self.filter.get_dimensions();
        let (filter_width, filter_height) = (filter_width as f64, filter_height as f64);
        let (window_width, window_height) = self.size.get();
        let (surface_width, surface_height) = target.get_dimensions();
        let (surface_width, surface_height) = (surface_width as f64, surface_height as f64);
        let filter_ratio = filter_width / filter_height;
        let window_ratio = window_width / window_height;
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
    }

    pub fn get_input(&self) -> HashMap<Key, bool> {
        self.process_events();
        self.input.borrow().clone()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

enum RendererMessage {
    Frame(Vec<u16>),
    Close,
}

pub struct Renderer {
    tx: Sender<RendererMessage>,
    input_rx: Receiver<(bool, HashMap<Key, bool>)>,
    closed: Cell<bool>,
    input: RefCell<HashMap<Key, bool>>,
}

impl Renderer {
    pub fn new<T: 'static + Filter + Send>(filter: T) -> Renderer {
        let (tx, rx) = channel();

        let (input_tx, input_rx) = channel();

        ::std::thread::spawn(move || {
            let gl = GliumRenderer::new(filter);

            loop {
                //Drop frames until we get to the most recent
                let mut frame = None;
                let mut empty = false;
                while !empty {
                    match rx.try_recv() {
                        Ok(RendererMessage::Frame(f)) => frame = Some(f),
                        Err(TryRecvError::Empty) => empty = true,
                        Err(TryRecvError::Disconnected) | Ok(RendererMessage::Close) => return,
                    }
                }

                if frame.is_some() {
                    gl.render(&frame.unwrap());
                    let _ = input_tx.send((gl.is_closed(), gl.get_input()));
                }
            }
        });

        Renderer {
            tx: tx,
            input_rx: input_rx,
            closed: Cell::new(false),
            input: RefCell::new(HashMap::new()),
        }
    }

    fn process_input(&self) {
        let mut input = None;
        let mut empty = false;
        while !empty {
            match self.input_rx.try_recv() {
                Ok(i) => input = Some(i),
                Err(TryRecvError::Empty) => empty = true,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if input.is_none() {
            return;
        }
        let input = input.unwrap();
        self.closed.set(input.0);
        *self.input.borrow_mut() = input.1;
    }

    pub fn add_frame(&self, frame: &[u16]) {
        let _ = self
            .tx
            .send(RendererMessage::Frame(frame.to_vec()))
            .unwrap();
        self.process_input();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }

    pub fn get_input(&self) -> HashMap<Key, bool> {
        self.input.clone().into_inner()
    }

    pub fn close(mut self) {
        self.closed = Cell::new(true);
        let _ = self.tx.send(RendererMessage::Close).unwrap();
    }
}
