use glium;
use glium::{DisplayBuild, Surface};
use glium::texture::{RawImage2d, RawImage1d, ClientFormat, texture2d, texture1d};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct GliumRenderer {
    display: glium::Display,
    indicies: glium::index::NoIndices,
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    closed: bool,
    palette: texture1d::Texture1d,
}

impl GliumRenderer {
    pub fn new(pal: &[u8; 192]) -> GliumRenderer {
        let display = glium::glutin::WindowBuilder::new()
            .with_dimensions(512, 480)
            .with_title(format!("Mass NES"))
            .build_glium()
            .unwrap();
        
        let top_right = Vertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] };
        let top_left = Vertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] };
        let bottom_left = Vertex { position: [-1.0, -1.0],  tex_coords: [0.0, 1.0] };
        let bottom_right = Vertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] };

        let shape = vec![top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let vertex_shader_src = r#"
            #version 140

            in vec2 position;
            in vec2 tex_coords;

            out vec2 v_tex_coords;

            void main() {
                v_tex_coords = tex_coords;
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            uniform sampler2D tex;
            uniform sampler1D palette;

            void main() {
                vec4 index = texture(tex, v_tex_coords);
                color = texture(palette, index.x * 4);
            }
        "#;

        let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let palette = RawImage1d {
            data: ::std::borrow::Cow::Borrowed(&pal[..]),
            width: 64,
            format: ClientFormat::U8U8U8,
        };

        let pal_tex = texture1d::Texture1d::new(&display, palette).unwrap();

        GliumRenderer {
            display: display,
            indicies: indicies,
            program: program,
            vertex_buffer: vertex_buffer,
            closed: false,
            palette: pal_tex,
        }
    }

    fn process_events(&mut self) {
        for ev in self.display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => {
                    self.closed = true;
                },
                glium::glutin::Event::KeyboardInput(state, _, key_opt) => {
                    let pressed = state == glium::glutin::ElementState::Pressed;
                }, 
                _ => {}
            }
        }
    }
    
    pub fn render(&mut self, screen: &[u8; 256*240]) {
        {
            let img = RawImage2d {
                data: ::std::borrow::Cow::Borrowed(screen),
                width: 256,
                height: 240,
                format: ClientFormat::U8,
            };

            let tex = texture2d::Texture2d::new(&self.display, img).unwrap();

            let uniforms = uniform! {
                tex: tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest).minify_filter(glium::uniforms::MinifySamplerFilter::Nearest),
                palette: self.palette.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest).minify_filter(glium::uniforms::MinifySamplerFilter::Nearest),
            };

            let mut target = self.display.draw(); 
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            target.draw(&self.vertex_buffer, &self.indicies, &self.program, &uniforms, &Default::default()).unwrap();
            target.finish().unwrap();
        }
        self.process_events();
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}
