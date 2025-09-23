use super::{
    Filter, FilterContext, FilterUniforms, Preprocessor, Program, TextureFilter, TextureFormat,
    TextureParams,
};

pub use nes_ntsc_rust::{NesNtsc, NesNtscSetup};

pub struct CrtFilter {
    program: Program<'static>,
    ntsc: NesNtsc,
    width: u32,
    height: u32,
    frame: Vec<u32>,
    merge_fields: bool,
    phase: i32,
}

impl CrtFilter {
    pub fn new(setup: &NesNtscSetup) -> Self {
        let merge_fields = setup.merge_fields;
        let width = NesNtsc::out_width(256);
        let height = 240;
        let ntsc = NesNtsc::new(setup);

        let processor = Preprocessor::new(super::CRT_SHADER);
        let program = processor.process().expect("valid shader source");

        Self {
            program,
            ntsc,
            width,
            height,
            frame: vec![0; (width * height) as usize],
            merge_fields,
            phase: 0,
        }
    }
}

impl<C: FilterContext> Filter<C> for CrtFilter {
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

        if !self.merge_fields {
            self.phase ^= 1;
        }

        self.ntsc
            .blit(screen, &mut self.frame, 256, 240, self.phase & 1);

        for p in self.frame.iter_mut() {
            let r = (*p & 0x00ff0000) >> 16;
            let b = (*p & 0x000000ff) << 16;
            *p = (*p & 0xff00ff00) | r | b;
        }

        let params = TextureParams {
            width: self.width as usize,
            height: self.height as usize,
            format: TextureFormat::RGBA,
            pixels: bytemuck::cast_slice(&self.frame),
            filter: TextureFilter::Nearest,
        };

        let texture = display.create_texture(params);

        let s_w = self.width as f32;
        let s_h = self.height as f32;
        let d_w = render_size.0 as f32;
        let d_h = render_size.1 as f32;

        unis.add_texture("Source", texture);
        unis.add_vec4("SourceSize", (s_w, s_h, 1.0 / s_w, 1.0 / s_h));
        unis.add_vec4("OriginalSize", (s_w, s_h, 1.0 / s_w, 1.0 / s_h));
        unis.add_vec4("OutputSize", (d_w, d_h, 1.0 / d_w, 1.0 / d_h));

        for p in self.program.parameters.iter() {
            unis.add_f32(p.name, p.value);
        }

        unis
    }

    fn parameters_mut(&mut self) -> std::slice::IterMut<'_, super::Parameter<'static>> {
        self.program.parameters.iter_mut()
    }
}
