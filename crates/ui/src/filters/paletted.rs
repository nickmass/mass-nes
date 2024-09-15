use super::{Filter, FilterContext, FilterUniforms, TextureFilter, TextureFormat, TextureParams};

pub struct PalettedFilter {
    palette: [u8; 1536],
}

impl PalettedFilter {
    pub fn new(palette: [u8; 1536]) -> PalettedFilter {
        PalettedFilter { palette }
    }
}

impl Filter for PalettedFilter {
    fn dimensions(&self) -> (u32, u32) {
        (256, 240)
    }

    fn vertex_shader(&self) -> &'static str {
        super::PALETTED_VERTEX_SHADER
    }

    fn fragment_shader(&self) -> &'static str {
        super::PALETTED_FRAGMENT_SHADER
    }

    fn process<C: FilterContext>(
        &mut self,
        display: &C,
        _render_size: (f64, f64),
        screen: &[u16],
    ) -> C::Uniforms {
        let (width, height) = self.dimensions();

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
