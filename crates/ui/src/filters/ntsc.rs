use super::{
    Filter, FilterContext, FilterUniforms, Preprocessor, Program, TextureFilter, TextureFormat,
    TextureParams,
};

pub use nes_ntsc_rust::{NesNtsc, NesNtscSetup};

pub struct NtscFilter {
    program: Program<'static>,
    ntsc: NesNtsc,
    width: u32,
    height: u32,
    frame: Vec<u32>,
}

impl NtscFilter {
    pub fn new(setup: &NesNtscSetup) -> NtscFilter {
        let processor = Preprocessor::new(super::NTSC_SHADER);
        let program = processor.process().expect("valid shader source");
        let width = NesNtsc::out_width(256);
        let height = 240;
        let ntsc = NesNtsc::new(setup);

        NtscFilter {
            program,
            ntsc,
            width,
            height,
            frame: vec![0; (width * height) as usize],
        }
    }
}

impl<C: FilterContext> Filter<C> for NtscFilter {
    fn dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 4)
    }

    fn vertex_shader(&self) -> &str {
        &self.program.vertex
    }

    fn fragment_shader(&self) -> &str {
        &self.program.fragment
    }

    fn process(&mut self, display: &C, render_size: (f64, f64), screen: &[u16]) -> C::Uniforms {
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
