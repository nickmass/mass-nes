use eframe::glow::{self, UniformLocation, VertexArray};
use glow::{Buffer, Context, HasContext};

use std::{cell::Cell, collections::HashMap, sync::Arc};

pub type GlowContext = Arc<Context>;

#[derive(Debug, Copy, Clone)]
pub enum Polygon {
    TriangleFan,
}

impl Polygon {
    fn as_gl_type(&self) -> u32 {
        match self {
            Polygon::TriangleFan => glow::TRIANGLE_FAN,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AttrType {
    Vec2,
}

impl AttrType {
    fn size(&self) -> i32 {
        match self {
            AttrType::Vec2 => 8,
        }
    }

    fn enable(&self, ctx: &GlowContext, location: u32) {
        unsafe {
            ctx.enable_vertex_attrib_array(location);
        }
    }

    fn layout(&self, ctx: &GlowContext, location: u32, stride: i32, offset: i32) {
        unsafe {
            match self {
                AttrType::Vec2 => {
                    ctx.vertex_attrib_pointer_f32(location, 2, glow::FLOAT, false, stride, offset);
                }
            }
        }
    }
}

pub trait Vertex: bytemuck::Pod {
    const ATTRIBUTES: &[(&'static str, AttrType)];
    const SIZE: usize;
}

pub struct VertexBuffer<V> {
    buffer: Buffer,
    poly_type: Polygon,
    count: i32,
    _marker: std::marker::PhantomData<V>,
}

impl<V: Vertex> VertexBuffer<V> {
    pub fn new(ctx: &GlowContext, poly_type: Polygon, data: &[V]) -> Result<Self, String> {
        unsafe {
            let ctx = ctx.clone();
            let buffer = ctx.create_buffer()?;
            ctx.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            ctx.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(data),
                glow::STATIC_DRAW,
            );
            ctx.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Self {
                buffer,
                poly_type,
                count: data.len() as i32,
                _marker: Default::default(),
            })
        }
    }

    fn enable_attrs(&self, ctx: &GlowContext, program: &glow::Program) {
        unsafe {
            let stride = V::ATTRIBUTES.iter().map(|a| a.1.size()).sum();
            let mut offset = 0;
            for (name, atype) in V::ATTRIBUTES {
                let Some(location) = ctx.get_attrib_location(*program, name) else {
                    tracing::warn!("attribute location not found for '{name}'");
                    continue;
                };
                atype.layout(ctx, location, stride, offset);
                atype.enable(ctx, location);
                offset += atype.size();
            }
        }
    }

    fn draw(&self, ctx: &GlowContext) {
        unsafe {
            ctx.draw_arrays(self.poly_type.as_gl_type(), 0, self.count);
        }
    }
}

pub struct Program {
    texture_unit: Cell<u32>,
    program: glow::Program,
    vao: Option<VertexArray>,
}

impl Program {
    pub fn new<V: AsRef<str>, F: AsRef<str>>(
        ctx: &GlowContext,
        vertex: V,
        fragment: F,
    ) -> Result<Self, String> {
        unsafe {
            let ctx = ctx.clone();
            let program = ctx.create_program()?;
            let vert_shader = ctx.create_shader(glow::VERTEX_SHADER)?;
            let frag_shader = ctx.create_shader(glow::FRAGMENT_SHADER)?;
            ctx.shader_source(vert_shader, vertex.as_ref());
            ctx.shader_source(frag_shader, fragment.as_ref());

            ctx.compile_shader(vert_shader);
            let log = ctx.get_shader_info_log(vert_shader);
            if log.len() > 0 {
                tracing::warn!("compiling vert shader: {log}");
            }
            ctx.compile_shader(frag_shader);
            let log = ctx.get_shader_info_log(frag_shader);
            if log.len() > 0 {
                tracing::warn!("compiling frag shader: {log}");
            }

            ctx.attach_shader(program, vert_shader);
            ctx.attach_shader(program, frag_shader);

            ctx.link_program(program);
            let linked = ctx.get_program_link_status(program);
            if !linked {
                tracing::warn!("program not linked");
            }
            let log = ctx.get_program_info_log(program);
            if log.len() > 0 {
                tracing::warn!("program: {log}");
            }

            ctx.detach_shader(program, vert_shader);
            ctx.delete_shader(vert_shader);
            ctx.detach_shader(program, frag_shader);
            ctx.delete_shader(frag_shader);

            Ok(Self {
                program,
                vao: None,
                texture_unit: Cell::new(0),
            })
        }
    }

    fn create_vao<V: Vertex>(&mut self, ctx: &GlowContext, vertex_buffer: &VertexBuffer<V>) {
        unsafe {
            let vao = ctx.create_vertex_array().ok();
            ctx.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer.buffer));
            ctx.bind_vertex_array(vao);
            vertex_buffer.enable_attrs(ctx, &self.program);
            self.vao = vao;
            ctx.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }

    pub fn draw<V: Vertex>(
        &mut self,
        ctx: &GlowContext,
        vertex_buffer: &VertexBuffer<V>,
        uniforms: &Uniforms,
    ) {
        unsafe {
            if let Some(vao) = self.vao.as_ref() {
                ctx.bind_vertex_array(Some(*vao));
            } else {
                self.create_vao(ctx, vertex_buffer);
            }
            ctx.use_program(Some(self.program));
            uniforms.bind(ctx, self);
            vertex_buffer.draw(ctx);
            self.reset_texture_unit();
        }
    }

    fn next_texture_unit(&self) -> u32 {
        let r = self.texture_unit.get();
        self.texture_unit.set(r + 1);
        r
    }

    fn reset_texture_unit(&self) {
        self.texture_unit.set(0);
    }

    pub fn delete(self, ctx: &GlowContext) {
        unsafe {
            if let Some(vao) = self.vao {
                ctx.delete_vertex_array(vao);
            }
            ctx.delete_program(self.program);
        }
    }
}

pub trait AsUniform {
    fn bind(&self, ctx: &GlowContext, program: &Program, location: Option<&UniformLocation>);

    fn delete(&self, _ctx: &GlowContext) {}
}

impl AsUniform for (f32, f32) {
    fn bind(&self, ctx: &GlowContext, _program: &Program, location: Option<&UniformLocation>) {
        unsafe {
            ctx.uniform_2_f32(location, self.0, self.1);
        }
    }
}

impl AsUniform for Texture {
    fn bind(&self, ctx: &GlowContext, program: &Program, location: Option<&UniformLocation>) {
        unsafe {
            let texture_unit = program.next_texture_unit();
            ctx.active_texture(glow::TEXTURE0 + texture_unit);
            ctx.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            ctx.uniform_1_i32(location, texture_unit as i32);
        }
    }

    fn delete(&self, ctx: &GlowContext) {
        unsafe {
            ctx.delete_texture(self.texture);
        }
    }
}

impl AsUniform for Box<dyn AsUniform> {
    fn bind(&self, ctx: &GlowContext, program: &Program, location: Option<&UniformLocation>) {
        (**self).bind(ctx, program, location)
    }

    fn delete(&self, ctx: &GlowContext) {
        (**self).delete(ctx);
    }
}

pub struct Uniforms {
    ctx: GlowContext,
    map: HashMap<&'static str, Box<dyn AsUniform>>,
}

impl Uniforms {
    pub fn new(ctx: &GlowContext) -> Self {
        Self {
            ctx: ctx.clone(),
            map: HashMap::new(),
        }
    }

    pub fn add<U: AsUniform + 'static>(&mut self, name: &'static str, value: U) {
        self.map.insert(name, Box::new(value));
    }

    fn bind(&self, ctx: &GlowContext, program: &Program) {
        for (name, value) in self.map.iter() {
            unsafe {
                let location = ctx.get_uniform_location(program.program, *name);

                if let Some(location) = location {
                    value.bind(ctx, program, Some(&location));
                } else {
                    tracing::warn!("uniform location for '{name}' not found.");
                }
            }
        }
    }
}

impl Drop for Uniforms {
    fn drop(&mut self) {
        for v in self.map.values() {
            v.delete(&self.ctx);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PixelFormat {
    RGBA,
    RGB,
    U16,
}

impl PixelFormat {
    fn format(&self) -> u32 {
        match self {
            PixelFormat::RGBA => glow::RGBA,
            PixelFormat::RGB => glow::RGB,
            PixelFormat::U16 => glow::RG_INTEGER,
        }
    }

    fn internal_format(&self) -> u32 {
        match self {
            PixelFormat::RGBA => glow::RGBA,
            PixelFormat::RGB => glow::RGB,
            PixelFormat::U16 => glow::RG8UI,
        }
    }

    fn ty(&self) -> u32 {
        match self {
            _ => glow::UNSIGNED_BYTE,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

impl Into<i32> for TextureFilter {
    fn into(self) -> i32 {
        match self {
            TextureFilter::Nearest => glow::NEAREST as i32,
            TextureFilter::Linear => glow::LINEAR as i32,
        }
    }
}

pub struct Texture {
    texture: glow::Texture,
}

impl Texture {
    pub fn new(
        ctx: &GlowContext,
        format: PixelFormat,
        width: u16,
        height: u16,
        pixels: &[u8],
    ) -> Result<Self, String> {
        unsafe {
            let texture = ctx.create_texture()?;
            ctx.bind_texture(glow::TEXTURE_2D, Some(texture));
            ctx.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                format.internal_format() as i32,
                width as i32,
                height as i32,
                0,
                format.format(),
                format.ty(),
                glow::PixelUnpackData::Slice(Some(pixels)),
            );

            ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            ctx.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );

            Ok(Self { texture })
        }
    }

    pub fn with_min_filter(self, ctx: &GlowContext, filter: TextureFilter) -> Texture {
        unsafe {
            ctx.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter.into());

            self
        }
    }

    pub fn with_mag_filter(self, ctx: &GlowContext, filter: TextureFilter) -> Texture {
        unsafe {
            ctx.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            ctx.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter.into());

            self
        }
    }
}
