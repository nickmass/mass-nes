use web_sys::HtmlCanvasElement;

use ui::filters::Filter;

use super::gl;

pub struct Gfx<T> {
    filter: T,
    canvas: HtmlCanvasElement,
    gl: gl::GlContext,
    screen: gl::GlModel<Vertex>,
    program: gl::GlProgram,
    size: (f64, f64),
    render_size: (u32, u32),
    resize_count: usize,
    frame: Option<Vec<u16>>,
}

impl<T: Filter> Gfx<T> {
    pub fn new(canvas: HtmlCanvasElement, filter: T) -> Self {
        let (width, height) = filter.dimensions();
        let size = (width as f64, height as f64);
        let render_size = (width, height);

        let gl = gl::GlContext::with_options(
            canvas.clone(),
            gl::WebGlContextOptions {
                alpha: false,
                depth: false,
                stencil: false,
                desynchronized: true,
                antialias: false,
                power_preference: gl::WebGlPowerPreference::HighPerformance,
            },
        );

        let top_right = Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 0.0],
        };
        let top_left = Vertex {
            position: [-1.0, 1.0],
            tex_coords: [0.0, 0.0],
        };
        let bottom_left = Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 1.0],
        };
        let bottom_right = Vertex {
            position: [1.0, -1.0],
            tex_coords: [1.0, 1.0],
        };

        let shape = [top_right, top_left, bottom_left, bottom_right];
        let screen = gl::GlModel::new(&gl, shape);
        let program = gl::GlProgram::new(&gl, filter.vertex_shader(), filter.fragment_shader());

        Self {
            filter,
            canvas,
            gl,
            screen,
            program,
            size,
            render_size,
            resize_count: 0,
            frame: None,
        }
    }

    pub fn resize(&mut self, (c_width, c_height): (u32, u32)) {
        let (width, height) = self.filter.dimensions();
        let (f_width, f_height) = (width as f64, height as f64);
        let (c_width, c_height) = (c_width as f64, c_height as f64);

        let (width, height) = if f_width < f_height {
            let ratio = f_height / f_width;
            (c_width, c_width * ratio)
        } else {
            let ratio = f_width / f_height;
            (c_height * ratio, c_height)
        };

        // high performance
        let render_scale = 1.0;
        let (new_width, new_height) = (
            (width * render_scale).floor() as u32,
            (height * render_scale).floor() as u32,
        );
        let (current_width, current_height) = (self.canvas.width(), self.canvas.height());

        // low performance
        //let (w, h) = self.filter.dimensions();
        //let (new_width, new_height) = (w / 2, h / 2);

        if new_width.abs_diff(current_width) < 5 && new_height.abs_diff(current_height) < 5 {
            return;
        }

        self.resize_count += 1;

        if self.resize_count > 1000 {
            tracing::debug!("resize");
        }

        self.canvas.set_width(new_width);
        self.canvas.set_height(new_height);
        self.render_size = (new_width, new_height);
        self.size = (width, height);
    }

    pub fn update_frame(&mut self, frame: Vec<u16>) {
        self.frame = Some(frame);
    }

    pub fn render(&mut self) {
        let Some(screen) = self.frame.as_ref() else {
            return;
        };

        let (render_width, render_height) = self.render_size;

        let uniforms = self.filter.process(
            &self.gl,
            (render_width as f64, render_height as f64),
            screen.as_ref(),
        );

        let (width, height) = (render_width as i32, render_height as i32);
        self.gl.viewport(0, 0, width, height);
        self.program.draw(&self.screen, &uniforms, None);
        self.gl.flush();
    }
}

impl ui::filters::FilterContext for gl::GlContext {
    type Uniforms = gl::GlUniformCollection;

    type Texture = gl::GlTexture;

    fn create_uniforms(&self) -> Self::Uniforms {
        gl::GlUniformCollection::new()
    }

    fn create_texture(&self, params: ui::filters::TextureParams) -> Self::Texture {
        let format = match params.format {
            ui::filters::TextureFormat::RGBA => gl::PixelFormat::RGBA,
            ui::filters::TextureFormat::RGB => gl::PixelFormat::RGB,
            ui::filters::TextureFormat::U16 => gl::PixelFormat::U16,
        };

        let filter = match params.filter {
            ui::filters::TextureFilter::Nearest => gl::TextureFilter::Nearest,
            ui::filters::TextureFilter::Linear => gl::TextureFilter::Linear,
        };

        let texture = gl::GlTexture::new(
            self,
            params.width as u32,
            params.height as u32,
            format,
            bytemuck::cast_slice(&params.pixels),
        )
        .with_min_filter(filter)
        .with_mag_filter(filter);

        texture
    }
}

impl ui::filters::FilterUniforms<gl::GlContext> for gl::GlUniformCollection {
    fn add_vec2(&mut self, name: &'static str, value: (f32, f32)) {
        self.add(name, value);
    }

    fn add_texture(&mut self, name: &'static str, value: gl::GlTexture) {
        self.add(name, value);
    }
}

struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl gl::AsGlVertex for Vertex {
    const ATTRIBUTES: &'static [(&'static str, gl::GlValueType)] = &[
        ("position", gl::GlValueType::Vec2),
        ("tex_coords", gl::GlValueType::Vec2),
    ];

    const POLY_TYPE: u32 = gl::GL::TRIANGLE_FAN;

    const SIZE: usize = std::mem::size_of::<Self>();

    fn write(&self, mut buf: impl std::io::Write) {
        use byteorder::{LittleEndian, WriteBytesExt};
        let _ = buf.write_f32::<LittleEndian>(self.position[0]);
        let _ = buf.write_f32::<LittleEndian>(self.position[1]);
        let _ = buf.write_f32::<LittleEndian>(self.tex_coords[0]);
        let _ = buf.write_f32::<LittleEndian>(self.tex_coords[1]);
    }
}
