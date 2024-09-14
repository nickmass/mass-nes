use nes_ntsc::{NesNtsc, NesNtscSetup};

use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};

use std::cell::{Cell, RefCell};

use crate::ui::gfx::{Filter, FilterScaling, FilterUniforms};

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
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width * 2, self.height * 4)
    }

    fn get_fragment_shader(&self) -> &'static str {
        include_str!("../../shaders/ntsc_frag.glsl")
    }

    fn get_vertex_shader(&self) -> &'static str {
        include_str!("../../shaders/ntsc_vert.glsl")
    }

    fn process<F: glium::backend::Facade>(
        &self,
        display: &F,
        render_size: (f64, f64),
        screen: &[u16],
    ) -> FilterUniforms {
        let mut unis = FilterUniforms::new();
        let mut out = self.frame.borrow_mut();
        self.phase.set(self.phase.get() ^ 1);
        self.ntsc
            .borrow_mut()
            .blit(256, screen, self.phase.get(), &mut *out, self.width * 4);

        let img = RawImage2d {
            data: ::std::borrow::Cow::Borrowed(&*out),
            width: self.width,
            height: self.height,
            format: ClientFormat::U8U8U8U8,
        };

        let tex =
            Texture2d::with_mipmaps(display, img, glium::texture::MipmapsOption::NoMipmap).unwrap();
        unis.add_2d_uniform("tex", tex, FilterScaling::Linear);
        unis.add("input_size", (self.width as f32, self.height as f32));
        unis.add("output_size", (render_size.0 as f32, render_size.1 as f32));

        unis
    }
}
