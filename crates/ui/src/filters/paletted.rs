use super::{
    Filter, FilterContext, FilterUniforms, Preprocessor, Program, TextureFilter, TextureFormat,
    TextureParams,
};

pub struct PalettedFilter {
    program: Program<'static>,
    palette: [u8; 1536],
}

impl PalettedFilter {
    pub fn new(palette: [u8; 1536]) -> PalettedFilter {
        let processor = Preprocessor::new(super::PALETTED_SHADER);
        let program = processor.process().expect("valid shader source");

        PalettedFilter { program, palette }
    }
}

impl<C: FilterContext> Filter<C> for PalettedFilter {
    fn dimensions(&self) -> (u32, u32) {
        (256, 240)
    }

    fn vertex_shader(&self) -> &str {
        &self.program.vertex
    }

    fn fragment_shader(&self) -> &str {
        &self.program.fragment
    }

    fn process(&mut self, display: &C, _render_size: (f64, f64), screen: &[u16]) -> C::Uniforms {
        let (width, height) = <Self as Filter<C>>::dimensions(self);

        let mut uniforms = display.create_uniforms();

        let tex_params = TextureParams {
            width: width as usize,
            height: height as usize,
            format: TextureFormat::U16,
            pixels: bytemuck::cast_slice(screen),
            filter: TextureFilter::Nearest,
        };

        let tex = display.create_texture(tex_params);
        uniforms.add_texture("nes_screen", tex);

        let pal_tex_params = TextureParams {
            width: 64,
            height: 8,
            format: TextureFormat::RGB,
            pixels: &self.palette,
            filter: TextureFilter::Nearest,
        };

        let pal_tex = display.create_texture(pal_tex_params);
        uniforms.add_texture("nes_palette", pal_tex);

        uniforms
    }
}
