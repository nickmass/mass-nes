use nes_ntsc::{NesNtsc, NesNtscSetup};

use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};

use std::cell::{Cell, RefCell};

use crate::ui::gfx::{Filter, FilterScaling, FilterUniforms};

pub struct NtscFilter {
    ntsc: Box<RefCell<NesNtsc>>,
    width: u32,
    height: u32,
    phase: Cell<u32>,
}

impl NtscFilter {
    pub fn new(setup: NesNtscSetup) -> NtscFilter {
        NtscFilter {
            ntsc: Box::new(RefCell::new(NesNtsc::new(setup))),
            width: NesNtsc::out_width(256),
            height: 240,
            phase: Cell::new(0),
        }
    }
}

impl Filter for NtscFilter {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 4)
    }

    fn get_fragment_shader(&self) -> String {
        r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            uniform sampler2D tex;

            void main() {
                float line_intensity = mod(v_tex_coords.y * 480.0, 2.0) * 0.1;
                vec3 col = texture(tex, v_tex_coords).zyx;
                color = vec4(col - (col * line_intensity), 1.0);
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
        let mut out = vec![0; (self.height * self.width) as usize];
        self.phase.set(self.phase.get() ^ 1);
        self.ntsc
            .borrow_mut()
            .blit(256, screen, self.phase.get(), &mut *out, self.width * 4);

        let img = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(&*out),
            width: self.width,
            height: self.height,
            format: ClientFormat::U8U8U8U8,
        };

        let tex = Texture2d::with_mipmaps(&*display, img, glium::texture::MipmapsOption::NoMipmap)
            .unwrap();
        unis.add_2d_uniform("tex".to_string(), tex, FilterScaling::Linear);

        unis
    }
}
