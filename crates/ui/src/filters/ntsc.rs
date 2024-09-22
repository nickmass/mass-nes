use super::{Filter, FilterContext, FilterUniforms, TextureFilter, TextureFormat, TextureParams};

pub use nes_ntsc_rust::{NesNtsc, NesNtscSetup};

pub struct NtscFilter {
    ntsc: NesNtsc,
    width: u32,
    height: u32,
    frame: Vec<u32>,
}

impl NtscFilter {
    pub fn new(setup: &NesNtscSetup) -> NtscFilter {
        let width = NesNtsc::out_width(256);
        let height = 240;
        let ntsc = NesNtsc::new(setup);

        NtscFilter {
            ntsc,
            width,
            height,
            frame: vec![0; (width * height) as usize],
        }
    }
}

impl Filter for NtscFilter {
    fn dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 4)
    }

    fn vertex_shader(&self) -> &'static str {
        super::NTSC_VERTEX_SHADER
    }

    fn fragment_shader(&self) -> &'static str {
        super::NTSC_FRAGMENT_SHADER
    }

    #[tracing::instrument(skip_all)]
    fn process<C: FilterContext>(
        &mut self,
        display: &C,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> C::Uniforms {
        let mut unis = display.create_uniforms();

        self.ntsc.blit(screen, &mut self.frame, 256, 240, 0);

        let params = TextureParams {
            width: self.width as usize,
            height: self.height as usize,
            format: TextureFormat::RGBA,
            pixels: bytemuck::cast_slice(&self.frame),
            filter: TextureFilter::Linear,
        };

        let texture = display.create_texture(params);

        unis.add_vec2("input_size", (self.width as f32, self.height as f32));
        unis.add_vec2("output_size", (render_size.0 as f32, render_size.1 as f32));
        unis.add_texture("nes_screen", texture);

        unis
    }
}
