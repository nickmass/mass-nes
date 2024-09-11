use glium::texture;
use glium::texture::integral_texture2d::IntegralTexture2d;
use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};
use glium::uniforms::UniformValue;

pub enum FilterScaling {
    Nearest,
    Linear,
}

enum FilterTexture {
    Texture2d(Texture2d),
    IntegralTexture2d(IntegralTexture2d),
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

pub struct FilterUniforms<'a> {
    uniforms: Vec<FilterUniform<'a>>,
}

impl<'a> FilterUniforms<'a> {
    pub fn new() -> FilterUniforms<'a> {
        FilterUniforms {
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

    pub fn add_i2d_uniform(&mut self, name: &'a str, tex: IntegralTexture2d, scale: FilterScaling) {
        let uni = FilterSampler {
            name,
            texture: FilterTexture::IntegralTexture2d(tex),
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

impl<'a> glium::uniforms::Uniforms for FilterUniforms<'a> {
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
                        FilterTexture::IntegralTexture2d(ref tex) => {
                            visit(
                                uni.name,
                                UniformValue::IntegralTexture2d(tex, Some(sampler)),
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

pub trait Filter {
    fn get_dimensions(&self) -> (u32, u32);
    fn get_fragment_shader(&self) -> &'static str;
    fn get_vertex_shader(&self) -> &'static str;
    fn process<F: glium::backend::Facade>(
        &self,
        display: &F,
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

    fn get_fragment_shader(&self) -> &'static str {
        include_str!("../shaders/paletted_frag.glsl")
    }

    fn get_vertex_shader(&self) -> &'static str {
        include_str!("../shaders/paletted_vert.glsl")
    }

    fn process<F: glium::backend::Facade>(
        &self,
        display: &F,
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
        unis.add_i2d_uniform("tex", tex, FilterScaling::Nearest);

        let palette = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(&self.palette[..]),
            width: 64,
            height: 8,
            format: ClientFormat::U8U8U8,
        };

        let pal_tex =
            Texture2d::with_mipmaps(&*display, palette, texture::MipmapsOption::NoMipmap).unwrap();
        unis.add_2d_uniform("palette", pal_tex, FilterScaling::Nearest);
        unis
    }
}
