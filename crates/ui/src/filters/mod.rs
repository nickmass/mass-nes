mod crt;
mod ntsc;
mod paletted;
mod preprocessor;

pub use crt::*;
pub use ntsc::*;
pub use paletted::*;
pub use preprocessor::*;

pub trait Filter<C: FilterContext> {
    fn dimensions(&self) -> (u32, u32);
    fn vertex_shader(&self) -> &str;
    fn fragment_shader(&self) -> &str;
    fn process(&mut self, ctx: &C, render_size: (f64, f64), screen: &[u16]) -> C::Uniforms;
    fn parameters_mut(&mut self) -> std::slice::IterMut<Parameter<'static>> {
        [].iter_mut()
    }
}

pub trait FilterContext: Sized {
    type Uniforms: FilterUniforms<Self>;
    type Texture;
    fn create_uniforms(&self) -> Self::Uniforms;
    fn create_texture(&self, params: TextureParams) -> Self::Texture;
}

pub trait FilterUniforms<C: FilterContext> {
    fn add_f32(&mut self, name: &'static str, value: f32);
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32));
    fn add_vec4(&mut self, name: &'static str, value: (f32, f32, f32, f32));
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

pub(crate) const PALETTED_SHADER: &'static str = include_str!("../../shaders/paletted.glsl");
pub(crate) const NTSC_SHADER: &'static str = include_str!("../../shaders/ntsc.glsl");
pub(crate) const CRT_SHADER: &'static str = include_str!("../../shaders/crt.glsl");

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const PRELUDE_SHADER: &'static str = include_str!("../../shaders/prelude_gl.glsl");
#[cfg(target_arch = "wasm32")]
pub(crate) const PRELUDE_SHADER: &'static str = include_str!("../../shaders/prelude_webgl.glsl");
