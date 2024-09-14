use nes_ntsc_c2rust::{NesNtsc, NesNtscSetup};

use std::cell::{RefCell, Cell};

use crate::gl;

pub trait Filter {
    fn dimensions(&self) -> (u32, u32);
    fn fragment_shader(&self) -> &'static str;
    fn vertex_shader(&self) -> &'static str;
    fn process(
        &mut self,
        display: &gl::GlContext,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> gl::GlUniformCollection ;
}

pub struct PalettedFilter {
    palette: [u8; 1536],
    palette_texture: Option<gl::GlTexture>,
    buf: Vec<u8>,
}

impl PalettedFilter {
    pub fn new(pal: [u8; 1536]) -> PalettedFilter {
        PalettedFilter { palette: pal, buf: Vec::new(), palette_texture: None }
    }
}

impl Filter for PalettedFilter {
    fn dimensions(&self) -> (u32, u32) {
        (256, 240)
    }

    fn fragment_shader(&self) -> &'static str {
        include_str!("../shaders/paletted_frag.glsl")
    }

    fn vertex_shader(&self) -> &'static str {
        include_str!("../shaders/paletted_vert.glsl")
    }

    fn process(
        &mut self,
        display: &gl::GlContext,
        _render_size: (f64, f64),
        screen: &[u16],
    ) -> gl::GlUniformCollection
    {
        let (width, height) = self.dimensions();

        let mut uniforms = gl::GlUniformCollection::new();
        let tex = gl::GlTexture::new(display, width, height, gl::PixelFormat::U16, bytemuck::cast_slice(screen));
        uniforms.add("nes_screen", tex);

        let pal_tex = gl::GlTexture::new(display, 64, 8, gl::PixelFormat::RGB, &self.palette);
        uniforms.add("nes_palette", pal_tex);

        uniforms
    }
}

pub struct NtscFilter {
    ntsc: Box<RefCell<NesNtsc>>,
    width: u32,
    height: u32,
    phase: Cell<u32>,
    frame: RefCell<Vec<u32>>,
}

impl NtscFilter {
    pub fn new(setup: NesNtscSetup) -> NtscFilter {
        let width = NesNtsc::out_width(256);
        let height = 240;
        NtscFilter {
            ntsc: Box::new(RefCell::new(NesNtsc::new(setup))),
            width,
            height,
            phase: Cell::new(0),
            frame: RefCell::new(vec![0; (width * height) as usize]),
        }
    }
}

impl Filter for NtscFilter {
    fn dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 4)
    }

    fn fragment_shader(&self) -> &'static str {
        include_str!("../shaders/ntsc_frag.glsl")
    }

    fn vertex_shader(&self) -> &'static str {
        include_str!("../shaders/ntsc_vert.glsl")
    }

    fn process(
        &mut self,
        display: &gl::GlContext,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> gl::GlUniformCollection
    {
        let mut unis = gl::GlUniformCollection::new();
        let mut out = self.frame.borrow_mut();
        self.phase.set(self.phase.get() ^ 1);
        self.ntsc
            .borrow_mut()
            .blit(256, screen, self.phase.get(), &mut *out, self.width * 4);

        let tex = gl::GlTexture::new(display, self.width, self.height, gl::PixelFormat::RGBA, bytemuck::cast_slice(&out))
            .with_min_filter(gl::TextureFilter::Linear)
            .with_mag_filter(gl::TextureFilter::Linear);

        unis.add("input_size", (self.width as f32, self.height as f32));
        unis.add("output_size", (render_size.0 as f32, render_size.1 as f32));
        unis.add("nes_screen", tex);

        unis
    }
}
