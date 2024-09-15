mod ntsc;
mod paletted;

pub use ntsc::*;
pub use paletted::*;

pub trait Filter {
    fn dimensions(&self) -> (u32, u32);
    fn vertex_shader(&self) -> &'static str;
    fn fragment_shader(&self) -> &'static str;
    fn process<C: FilterContext>(
        &mut self,
        ctx: &C,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> C::Uniforms;
}

pub trait FilterContext: Sized {
    type Uniforms: FilterUniforms<Self>;
    type Texture;
    fn create_uniforms(&self) -> Self::Uniforms;
    fn create_texture(&self, params: TextureParams) -> Self::Texture;
}

pub trait FilterUniforms<C: FilterContext> {
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32));
    fn add_texture(&mut self, name: &'static str, value: C::Texture);
}

#[derive(Debug, Clone)]
pub struct TextureParams<'a> {
    pub width: usize,
    pub height: usize,
    pub format: TextureFormat,
    pub pixels: &'a [u8],
    pub filter: TextureFilter,
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFormat {
    RGBA,
    RGB,
    U16,
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! shader(
    ($name:literal) => {
        concat!(include_str!(concat!("../../shaders/prelude_gl.glsl")), include_str!(concat!("../../shaders/", $name)))
    }
);

#[cfg(target_arch = "wasm32")]
macro_rules! shader(
    ($name:literal) => {
        concat!(include_str!(concat!("../../shaders/prelude_webgl.glsl")), include_str!(concat!("../../shaders/", $name)))
    }
);

pub(crate) const PALETTED_VERTEX_SHADER: &'static str = shader!("paletted_vert.glsl");
pub(crate) const PALETTED_FRAGMENT_SHADER: &'static str = shader!("paletted_frag.glsl");
pub(crate) const NTSC_VERTEX_SHADER: &'static str = shader!("ntsc_vert.glsl");
pub(crate) const NTSC_FRAGMENT_SHADER: &'static str = shader!("ntsc_frag.glsl");
