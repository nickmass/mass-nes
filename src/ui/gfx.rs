use glium::texture;
use glium::texture::integral_texture2d::IntegralTexture2d;
use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};

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
            name,
            texture: FilterTexture::Texture2d(tex),
            scaling: scale,
        };

        self.uniforms.push(uni);
    }

    pub fn add_i2d_uniform(&mut self, name: String, tex: IntegralTexture2d, scale: FilterScaling) {
        let uni = FilterUniform {
            name,
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
