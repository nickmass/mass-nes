use super::{Filter, FilterContext, FilterUniforms, TextureFilter, TextureFormat, TextureParams};

#[cfg(target_arch = "wasm32")]
use nes_ntsc_c2rust as nes_ntsc;

pub use nes_ntsc::{NesNtsc, NesNtscSetup};

pub struct NtscFilter {
    ntsc: Box<NesNtsc>,
    width: u32,
    height: u32,
    phase: u32,
    frame: Vec<u32>,
}

impl NtscFilter {
    pub fn new(setup: NesNtscSetup) -> NtscFilter {
        let width = NesNtsc::out_width(256);
        let height = 240;
        NtscFilter {
            ntsc: Box::new(NesNtsc::new(setup)),
            width,
            height,
            phase: 0,
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

    fn process<C: FilterContext>(
        &mut self,
        display: &C,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> C::Uniforms {
        let mut unis = display.create_uniforms();
        self.phase = self.phase ^ 1;
        self.ntsc
            .blit(256, screen, self.phase, &mut self.frame, self.width * 4);

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
