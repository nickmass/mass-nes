use glium;
use glium::{DisplayBuild, Surface};
use glium::texture::{RawImage2d, ClientFormat};
use glium::texture::integral_texture2d::IntegralTexture2d;
use glium::texture::texture2d::Texture2d;
use glium::texture;

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
    palette: Texture2d,
    input: [bool; 8],
}

impl GliumRenderer {
    pub fn new(pal: &[u8; 1536]) -> GliumRenderer {
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

            uniform isampler2D tex;
            uniform sampler2D palette;

            void main() {
                ivec4 index = texture(tex, v_tex_coords);
                color = texelFetch(palette, ivec2(index.x % 64, index.x / 64), 0);
            }
        "#;

        let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let palette = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(&pal[..]),
            width: 64,
            height: 8,
            format: ClientFormat::U8U8U8,
        };

        let pal_tex = Texture2d::with_mipmaps(&display, palette, 
                                        texture::MipmapsOption::NoMipmap).unwrap();

        GliumRenderer {
            display: display,
            indicies: indicies,
            program: program,
            vertex_buffer: vertex_buffer,
            closed: false,
            palette: pal_tex,
            input: [false; 8],
        }
    }

    fn process_events(&mut self) {
        for ev in self.display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => {
                    self.closed = true;
                },
                glium::glutin::Event::KeyboardInput(state, _, key_opt) => {
                    let mut input = self.input;
                    let pressed = state == glium::glutin::ElementState::Pressed;
                    if let Some(key) = key_opt {
                        use glium::glutin::VirtualKeyCode as K;
                        match key {
                            K::Z => input[0] = pressed,
                            K::X => input[1] = pressed,
                            K::RShift => input[2] = pressed,
                            K::Return => input[3] = pressed,
                            K::Up => input[4] = pressed,
                            K::Down => input[5] = pressed,
                            K::Left => input[6] = pressed,
                            K::Right => input[7] = pressed,
                             _ => {}
                        }
                    }
                    self.input = input;
                }, 
                _ => {}
            }
        }
    }

    pub fn render(&mut self, screen: &[u16; 256*240]) {
        {
            let img = RawImage2d {
                data: ::std::borrow::Cow::Borrowed(screen),
                width: 256,
                height: 240,
                format: ClientFormat::U16,
            };

            use glium::{uniforms, texture};
            let tex = IntegralTexture2d::with_mipmaps(
                &self.display, img, texture::MipmapsOption::NoMipmap).unwrap();

            let uniforms = uniform! {
                tex: tex.sampled().magnify_filter(uniforms::MagnifySamplerFilter::Nearest).minify_filter(uniforms::MinifySamplerFilter::Nearest),
                palette: self.palette.sampled().magnify_filter(uniforms::MagnifySamplerFilter::Nearest).minify_filter(uniforms::MinifySamplerFilter::Nearest),
            };

            let mut target = self.display.draw(); 
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            target.draw(&self.vertex_buffer, &self.indicies, &self.program, &uniforms, &Default::default()).unwrap();
            target.finish().unwrap();
        }
        self.process_events();
    }
   
    pub fn get_input(&self) -> [bool; 8] {
        self.input
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
}
