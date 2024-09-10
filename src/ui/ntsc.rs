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
    frame: RefCell<Vec<u32>>,
}

impl NtscFilter {
    pub fn new(setup: NesNtscSetup) -> NtscFilter {
        let width = NesNtsc::out_width(256);
        let height = 240;
        NtscFilter {
            ntsc: Box::new(RefCell::new(NesNtsc::new(setup))),
            width,
            height,
            phase: Cell::new(0),
            frame: RefCell::new(vec![0; (width * height) as usize]),
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

            // Adapted from https://www.shadertoy.com/view/WsVSzV

            float warp = 0.4; // simulate curvature of CRT monitor
            float scan = 0.75; // simulate darkness between scanlines

            in vec2 v_tex_coords;
            out vec4 color;

            uniform vec2 input_size;
            uniform vec2 output_size;
            uniform sampler2D tex;

            vec4 sharp_bilinear(sampler2D tex, vec2 uv)
            {
                vec2 texel = uv * input_size;
                vec2 scale = max(floor(output_size / input_size), vec2(1.0, 1.0));

                vec2 texel_floored = floor(texel);
                vec2 s = fract(texel);
                vec2 region_range = 0.5 - 0.5 / scale;

                vec2 center_dist = s - 0.5;
                vec2 f = (center_dist - clamp(center_dist, -region_range, region_range)) * scale + 0.5;

                vec2 mod_texel = texel_floored + f;

                return texture(tex, mod_texel / input_size);
            }

            void main()
            {
                // squared distance from center
                vec2 uv = v_tex_coords.xy;
                vec2 dc = abs(0.5-uv);
                dc *= dc;

                // warp the fragment coordinates
                uv.x -= 0.5; uv.x *= 1.0+(dc.y*(0.3*warp)); uv.x += 0.5;
                uv.y -= 0.5; uv.y *= 1.0+(dc.x*(0.4*warp)); uv.y += 0.5;

                // sample inside boundaries, otherwise set to black
                if (uv.y > 1.0 || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0)
                    color = vec4(0.0,0.0,0.0,1.0);
                else
                {
                    // determine if we are drawing in a scanline
                    float apply = abs(sin(v_tex_coords.y * input_size.y * 4.0)*0.5*scan);
                    // sample the texture
                    color = vec4(mix(sharp_bilinear(tex, uv).zyx,vec3(0.0),apply),1.0);
                }
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

    fn process<F: glium::backend::Facade>(
        &self,
        display: &F,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> FilterUniforms {
        let mut unis = FilterUniforms::new();
        let mut out = self.frame.borrow_mut();
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

        let tex =
            Texture2d::with_mipmaps(display, img, glium::texture::MipmapsOption::NoMipmap).unwrap();
        unis.add_2d_uniform("tex".to_string(), tex, FilterScaling::Linear);
        unis.add("input_size", (self.width as f32, self.height as f32));
        unis.add("output_size", (render_size.0 as f32, render_size.1 as f32));

        unis
    }
}
