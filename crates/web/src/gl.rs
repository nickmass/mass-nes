#![allow(dead_code)]
use wasm_bindgen::prelude::*;
use web_sys::{
    js_sys, wasm_bindgen, OffscreenCanvas, WebGlBuffer, WebGlFramebuffer, WebGlProgram,
    WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

pub use web_sys::WebGl2RenderingContext as GL;

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

static mut MODEL_ID: u64 = 0;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebGlContextOptions {
    pub alpha: bool,
    pub depth: bool,
    pub stencil: bool,
    pub desynchronized: bool,
    pub antialias: bool,
    pub power_preference: WebGlPowerPreference,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WebGlPowerPreference {
    Default,
    HighPerformance,
    LowPower,
}

#[derive(Clone)]
pub struct GlContext<C: Clone = OffscreenCanvas> {
    gl: GL,
    canvas: C,
    ext_map: Rc<RefCell<HashMap<TypeId, Option<Box<dyn Any>>>>>,
}

impl GlContext {
    pub fn new(canvas: OffscreenCanvas) -> Self {
        let gl = canvas
            .get_context("webgl2")
            .unwrap_or(None)
            .and_then(|e| e.dyn_into::<GL>().ok())
            .unwrap();
        GlContext::with_gl(canvas, gl)
    }

    pub fn with_options(
        canvas: OffscreenCanvas,
        options: WebGlContextOptions,
    ) -> GlContext<OffscreenCanvas> {
        let opts = serde_json::to_string(&options).unwrap();
        let opts = js_sys::JSON::parse(&opts).unwrap();
        let gl = canvas
            .get_context_with_context_options("webgl2", &opts)
            .unwrap_or(None)
            .and_then(|e| e.dyn_into::<GL>().ok())
            .unwrap();
        GlContext::with_gl(canvas, gl)
    }
}

impl<C: Clone> GlContext<C> {
    pub fn with_gl(canvas: C, gl: GL) -> Self {
        GlContext {
            gl,
            canvas,
            ext_map: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn canvas(&self) -> &C {
        &self.canvas
    }

    pub fn load_extension<E: GlExtension>(&self) -> Option<E> {
        let key = TypeId::of::<E>();
        let mut map = self.ext_map.borrow_mut();
        let entry = map.entry(key).or_insert_with(|| {
            self.gl
                .get_extension(E::EXT_NAME)
                .transpose()
                .and_then(|r| r.ok())
                .map(|e| Box::new(e.unchecked_into::<E>()) as Box<dyn Any>)
        });

        entry.as_ref().and_then(|e| e.downcast_ref::<E>()).cloned()
    }
}

impl<C: Clone> std::ops::Deref for GlContext<C> {
    type Target = GL;

    fn deref(&self) -> &Self::Target {
        &self.gl
    }
}

pub trait GlExtension: Any + Clone + JsCast {
    const EXT_NAME: &'static str;
}

impl GlExtension for OESElementIndexUint {
    const EXT_NAME: &'static str = "OES_element_index_uint";
}

pub struct GlProgram {
    gl: GlContext,
    program: WebGlProgram,
    vertex_shader: WebGlShader,
    fragment_shader: WebGlShader,
    texture_unit: Cell<u32>,
    vao_map: HashMap<u64, WebGlVertexArrayObject>,
    uniform_map: HashMap<&'static str, Option<WebGlUniformLocation>>,
}

impl GlProgram {
    pub fn new(
        gl: &GlContext,
        vertex_shader: impl AsRef<str>,
        fragment_shader: impl AsRef<str>,
    ) -> GlProgram {
        let gl = gl.clone();
        let shader_vert = gl
            .create_shader(GL::VERTEX_SHADER)
            .expect("Valid Vertex Shader");
        gl.shader_source(&shader_vert, vertex_shader.as_ref());
        gl.compile_shader(&shader_vert);
        let info = gl.get_shader_info_log(&shader_vert);
        if let Some(info) = info {
            if info.len() > 0 {
                tracing::warn!("Vertex Shader: {}\n{}", info, vertex_shader.as_ref());
            }
        }

        let shader_frag = gl
            .create_shader(GL::FRAGMENT_SHADER)
            .expect("Valid Fragment Shader");
        gl.shader_source(&shader_frag, fragment_shader.as_ref());
        gl.compile_shader(&shader_frag);
        let info = gl.get_shader_info_log(&shader_frag);
        if let Some(info) = info {
            if info.len() > 0 {
                tracing::warn!("Fragment Shader: {}\n{}", info, fragment_shader.as_ref());
            }
        }

        let prog = gl.create_program().expect("Create GL Program");
        gl.attach_shader(&prog, &shader_vert);
        gl.attach_shader(&prog, &shader_frag);
        gl.link_program(&prog);

        let info = gl.get_program_info_log(&prog);
        if let Some(info) = info {
            if info.len() > 0 {
                tracing::warn!(
                    "Program Shader: {} {} {}",
                    info,
                    vertex_shader.as_ref(),
                    fragment_shader.as_ref()
                );
            }
        }

        GlProgram {
            gl,
            program: prog,
            texture_unit: Cell::new(0),
            vertex_shader: shader_vert,
            fragment_shader: shader_frag,
            vao_map: HashMap::new(),
            uniform_map: HashMap::new(),
        }
    }

    pub fn draw<V>(
        &mut self,
        model: &GlModel<V>,
        uniforms: &GlUniformCollection,
        range: Option<Range<usize>>,
    ) where
        V: AsGlVertex,
    {
        self.draw_indexed::<_, u16>(model, uniforms, None, range)
    }

    pub fn draw_indexed<V, B>(
        &mut self,
        model: &GlModel<V>,
        uniforms: &GlUniformCollection,
        indices: Option<&GlIndexBuffer<B>>,
        range: Option<Range<usize>>,
    ) where
        V: AsGlVertex,
        B: GlIndex,
    {
        self.gl.use_program(Some(&self.program));

        let key = model.id;
        if let Some(vao) = self.vao_map.get(&key) {
            self.gl.bind_vertex_array(Some(vao))
        } else {
            let vao = self.gl.create_vertex_array().expect("Create vao");
            self.vao_map.insert(key, vao);
            let vao = self.vao_map.get(&key).unwrap();

            self.gl.bind_vertex_array(Some(vao));
            model.fill_vao(&self);
        }

        self.bind_uniforms(uniforms);
        model.draw(indices, range);

        self.gl.bind_vertex_array(None);
        self.reset_texture_unit();
    }

    pub fn draw_instanced<V, I>(
        &mut self,
        model: &GlModel<V>,
        instanced_data: impl IntoIterator<Item = I, IntoIter = impl ExactSizeIterator<Item = I>>,
        uniforms: &GlUniformCollection,
    ) where
        V: AsGlVertex,
        I: AsGlVertex,
    {
        self.gl.use_program(Some(&self.program));

        let key = model.id;
        if let Some(vao) = self.vao_map.get(&key) {
            self.gl.bind_vertex_array(Some(vao));
        } else {
            let vao = self.gl.create_vertex_array().expect("Create vao");
            self.vao_map.insert(key, vao);
            let vao = self.vao_map.get(&key).unwrap();

            self.gl.bind_vertex_array(Some(vao));
            model.fill_vao_instanced::<I>(&self);
        }

        self.bind_uniforms(uniforms);
        model.draw_instanced(instanced_data);

        self.gl.bind_vertex_array(None);
        self.reset_texture_unit();
    }

    fn bind_uniforms(&mut self, uniforms: &GlUniformCollection) {
        for (k, v) in &uniforms.uniforms {
            let location = if let Some(location) = self.uniform_map.get(k) {
                location
            } else {
                let location = self.gl.get_uniform_location(&self.program, k);
                self.uniform_map.insert(k, location);
                self.uniform_map.get(k).unwrap()
            };
            if location.is_some() {
                v.bind(&self.gl, &self, location.as_ref());
            }
        }
    }

    fn next_texture_unit(&self) -> u32 {
        let r = self.texture_unit.get();
        self.texture_unit.set(r + 1);
        r
    }

    fn reset_texture_unit(&self) {
        self.texture_unit.set(0)
    }
}

impl Drop for GlProgram {
    fn drop(&mut self) {
        for (_k, v) in self.vao_map.drain() {
            let _ = self.gl.delete_vertex_array(Some(&v));
        }

        self.gl.detach_shader(&self.program, &self.vertex_shader);
        self.gl.detach_shader(&self.program, &self.fragment_shader);

        self.gl.delete_shader(Some(&self.vertex_shader));
        self.gl.delete_shader(Some(&self.fragment_shader));

        self.gl.delete_program(Some(&self.program));
    }
}

pub struct GlUniformCollection {
    uniforms: HashMap<&'static str, Box<dyn AsGlUniform>>,
}

impl GlUniformCollection {
    pub fn new() -> GlUniformCollection {
        GlUniformCollection {
            uniforms: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            uniforms: HashMap::with_capacity(capacity),
        }
    }

    pub fn add<T: AsGlUniform + 'static>(&mut self, name: &'static str, uniform: T) -> &mut Self {
        self.uniforms.insert(name, Box::new(uniform));

        self
    }
}

impl AsGlUniform for Box<dyn AsGlUniform + '_> {
    fn bind(&self, gl: &GL, program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        (**self).bind(gl, program, location)
    }
}

impl<'a, T: AsGlUniform + ?Sized> AsGlUniform for &'a T {
    fn bind(&self, gl: &GL, program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        (*self).bind(gl, program, location)
    }
}

pub trait AsGlUniform {
    fn bind(&self, gl: &GL, program: &GlProgram, location: Option<&WebGlUniformLocation>);
}

impl AsGlUniform for bool {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform1f(location, if *self { 1.0 } else { 0.0 });
    }
}

impl AsGlUniform for i32 {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform1i(location, *self);
    }
}

impl AsGlUniform for f32 {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform1f(location, *self);
    }
}

impl AsGlUniform for [f32; 2] {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform2fv_with_f32_array(location, &self[..]);
    }
}

impl AsGlUniform for (f32, f32) {
    fn bind(&self, gl: &GL, program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        [self.0, self.1].bind(gl, program, location);
    }
}

impl AsGlUniform for [f32; 3] {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform3fv_with_f32_array(location, &self[..]);
    }
}

impl AsGlUniform for [f32; 4] {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform4fv_with_f32_array(location, &self[..]);
    }
}

impl AsGlUniform for [f32; 9] {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform_matrix3fv_with_f32_array(location, false, &self[..]);
    }
}

impl AsGlUniform for Vec<f32> {
    fn bind(&self, gl: &GL, _program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        gl.uniform1fv_with_f32_array(location, &self[..]);
    }
}

impl AsGlUniform for GlTexture {
    fn bind(&self, gl: &GL, program: &GlProgram, location: Option<&WebGlUniformLocation>) {
        let texture_unit = program.next_texture_unit();
        gl.active_texture(GL::TEXTURE0 + texture_unit);
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        gl.uniform1i(location, texture_unit as i32);
    }
}

pub struct GlModel<V: AsGlVertex> {
    gl: GlContext,
    id: u64,
    data: Vec<u8>,
    buffer: WebGlBuffer,
    instanced_buffer: WebGlBuffer,
    poly_type: u32,
    poly_count: i32,
    _marker: std::marker::PhantomData<V>,
}

impl<V: AsGlVertex> GlModel<V> {
    pub fn new(
        gl: &GlContext,
        vertexes: impl IntoIterator<Item = V, IntoIter = impl ExactSizeIterator<Item = V>>,
    ) -> GlModel<V> {
        let mut model = Self::empty(gl);
        model.fill(vertexes);
        model
    }

    pub fn empty(gl: &GlContext) -> GlModel<V> {
        let gl = gl.clone();
        let buffer = gl.create_buffer().expect("Gl Buffer");

        let (poly_type, poly_count) = (V::POLY_TYPE, 0);

        let instanced_buffer = gl.create_buffer().expect("Gl Instance Buffer");

        let id = unsafe {
            MODEL_ID += 1;
            MODEL_ID
        };

        GlModel {
            gl,
            id,
            data: Vec::new(),
            buffer,
            poly_type,
            poly_count,
            instanced_buffer,
            _marker: Default::default(),
        }
    }

    pub fn fill<A: std::borrow::Borrow<V>>(
        &mut self,
        vertexes: impl IntoIterator<Item = A, IntoIter = impl ExactSizeIterator<Item = A>>,
    ) {
        self.data.clear();
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.buffer));

        let vertexes = vertexes.into_iter();
        self.poly_count = vertexes.len() as i32;

        let data_size = vertexes.len() * V::SIZE;
        if data_size > self.data.capacity() {
            self.data.reserve(data_size - self.data.capacity());
        }

        for v in vertexes {
            v.borrow().write(&mut self.data);
        }

        self.gl
            .buffer_data_with_u8_array(GL::ARRAY_BUFFER, self.data.as_slice(), GL::DYNAMIC_DRAW);
    }

    fn fill_vao(&self, program: &GlProgram) {
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.buffer));
        self.enable_attrs::<V>(program, None);
    }

    fn fill_vao_instanced<I: AsGlVertex>(&self, program: &GlProgram) {
        self.gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.buffer));
        self.enable_attrs::<V>(program, Some(0));
        self.gl
            .bind_buffer(GL::ARRAY_BUFFER, Some(&self.instanced_buffer));
        self.enable_attrs::<I>(program, Some(1));
    }

    fn enable_attrs<I: AsGlVertex>(&self, program: &GlProgram, divisor: Option<u32>) {
        let stride = I::ATTRIBUTES.iter().map(|a| a.1.size()).sum();
        let mut offset = 0;
        for (name, vtype) in I::ATTRIBUTES {
            let location = self.gl.get_attrib_location(&program.program, name);
            if location < 0 {
                continue;
            }
            let location = location as u32;
            if let Some(divisor) = divisor {
                for i in 0..vtype.elements() {
                    self.gl.vertex_attrib_divisor(location + i, divisor);
                }
            }
            vtype.layout(&self.gl, location, stride, offset);
            vtype.enable(&self.gl, location);
            offset += vtype.size();
        }
    }

    fn draw<B: GlIndex>(&self, indices: Option<&GlIndexBuffer<B>>, range: Option<Range<usize>>) {
        let offset = range.as_ref().map(|r| r.start as i32);
        let count = range.as_ref().map(|r| (r.end - r.start) as i32);

        if let Some(indices) = indices {
            indices.bind();
            self.gl.draw_elements_with_i32(
                self.poly_type,
                count.unwrap_or(indices.length as i32),
                B::INDEX_TYPE,
                offset.unwrap_or(0),
            );
        } else {
            self.gl.draw_arrays(
                self.poly_type,
                offset.unwrap_or(0),
                count.unwrap_or(self.poly_count),
            );
        }
    }

    fn draw_instanced<I: AsGlVertex>(
        &self,
        instance_vertexes: impl IntoIterator<Item = I, IntoIter = impl ExactSizeIterator<Item = I>>,
    ) {
        self.gl
            .bind_buffer(GL::ARRAY_BUFFER, Some(&self.instanced_buffer));

        let iter = instance_vertexes.into_iter();
        let count = iter.len();
        let mut data = Vec::with_capacity(count * I::SIZE);
        for v in iter {
            v.write(&mut data);
        }

        self.gl
            .buffer_data_with_u8_array(GL::ARRAY_BUFFER, data.as_slice(), GL::DYNAMIC_DRAW);

        self.gl
            .draw_arrays_instanced(self.poly_type, 0, self.poly_count, count as i32)
    }
}

impl<V: AsGlVertex> Drop for GlModel<V> {
    fn drop(&mut self) {
        self.gl.delete_buffer(Some(&self.buffer));
        self.gl.delete_buffer(Some(&self.instanced_buffer));
    }
}

pub trait GlIndex: Sized {
    const INDEX_TYPE: u32;

    type Array: std::ops::Deref<Target = js_sys::Object>;

    fn create_array<'a>(data: &'a [Self]) -> IndexArray<'a, Self, Self::Array>;
}

impl GlIndex for u16 {
    const INDEX_TYPE: u32 = GL::UNSIGNED_SHORT;

    type Array = js_sys::Uint16Array;

    fn create_array<'a>(data: &'a [u16]) -> IndexArray<'a, Self, Self::Array> {
        unsafe { IndexArray(data, js_sys::Uint16Array::view(data)) }
    }
}

impl GlIndex for u32 {
    const INDEX_TYPE: u32 = GL::UNSIGNED_INT;

    type Array = js_sys::Uint32Array;

    fn create_array<'a>(data: &'a [u32]) -> IndexArray<'a, Self, Self::Array> {
        unsafe { IndexArray(data, js_sys::Uint32Array::view(data)) }
    }
}

pub struct IndexArray<'a, D, T>(&'a [D], T);

pub struct GlIndexBuffer<T: GlIndex> {
    gl: GlContext,
    buffer: WebGlBuffer,
    length: usize,
    marker: std::marker::PhantomData<T>,
}

impl<T: GlIndex> GlIndexBuffer<T> {
    pub fn new(gl: &GlContext, indices: &[T]) -> Self {
        let mut buffer = Self::empty(gl);
        buffer.fill(indices);

        buffer
    }

    pub fn empty(gl: &GlContext) -> Self {
        let gl = gl.clone();
        let buffer = gl.create_buffer().expect("Create Index Buffer");

        Self {
            gl,
            buffer,
            length: 0,
            marker: Default::default(),
        }
    }

    fn bind(&self) {
        self.gl
            .bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.buffer));
    }

    pub fn fill(&mut self, indices: &[T]) {
        self.bind();

        let bytes = T::create_array(indices);
        self.gl.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &bytes.1,
            GL::DYNAMIC_DRAW,
        );

        self.length = indices.len();
    }
}

impl<T: GlIndex> Drop for GlIndexBuffer<T> {
    fn drop(&mut self) {
        self.gl.delete_buffer(Some(&self.buffer));
    }
}

pub trait AsGlVertex {
    const ATTRIBUTES: &'static [(&'static str, GlValueType)];
    const POLY_TYPE: u32;
    const SIZE: usize;

    fn write(&self, buf: impl std::io::Write);
}

pub enum GlValueType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Mat3,
    Mat4,
}

impl GlValueType {
    fn size(&self) -> i32 {
        match self {
            GlValueType::Float => 4,
            GlValueType::Vec2 => 8,
            GlValueType::Vec3 => 12,
            GlValueType::Vec4 => 16,
            GlValueType::Mat3 => 36,
            GlValueType::Mat4 => 64,
        }
    }

    fn elements(&self) -> u32 {
        match self {
            GlValueType::Mat3 => 3,
            GlValueType::Mat4 => 4,
            _ => 1,
        }
    }

    fn enable(&self, gl: &GL, location: u32) {
        match self {
            GlValueType::Mat3 => {
                gl.enable_vertex_attrib_array(location);
                gl.enable_vertex_attrib_array(location + 1);
                gl.enable_vertex_attrib_array(location + 2);
            }
            GlValueType::Mat4 => {
                gl.enable_vertex_attrib_array(location);
                gl.enable_vertex_attrib_array(location + 1);
                gl.enable_vertex_attrib_array(location + 2);
                gl.enable_vertex_attrib_array(location + 3);
            }
            _ => gl.enable_vertex_attrib_array(location),
        }
    }

    fn disable(&self, gl: &GL, location: u32) {
        match self {
            GlValueType::Mat3 => {
                gl.disable_vertex_attrib_array(location);
                gl.disable_vertex_attrib_array(location + 1);
                gl.disable_vertex_attrib_array(location + 2);
            }
            GlValueType::Mat4 => {
                gl.disable_vertex_attrib_array(location);
                gl.disable_vertex_attrib_array(location + 1);
                gl.disable_vertex_attrib_array(location + 2);
                gl.disable_vertex_attrib_array(location + 3);
            }
            _ => gl.disable_vertex_attrib_array(location),
        }
    }

    fn layout(&self, gl: &GL, location: u32, stride: i32, offset: i32) {
        match self {
            GlValueType::Float => {
                gl.vertex_attrib_pointer_with_i32(location, 1, GL::FLOAT, false, stride, offset);
            }
            GlValueType::Vec2 => {
                gl.vertex_attrib_pointer_with_i32(location, 2, GL::FLOAT, false, stride, offset);
            }
            GlValueType::Vec3 => {
                gl.vertex_attrib_pointer_with_i32(location, 3, GL::FLOAT, false, stride, offset);
            }
            GlValueType::Vec4 => {
                gl.vertex_attrib_pointer_with_i32(location, 4, GL::FLOAT, false, stride, offset);
            }
            GlValueType::Mat3 => {
                gl.vertex_attrib_pointer_with_i32(location, 3, GL::FLOAT, false, stride, offset);
                gl.vertex_attrib_pointer_with_i32(
                    location + 1,
                    3,
                    GL::FLOAT,
                    false,
                    stride,
                    offset + 12,
                );
                gl.vertex_attrib_pointer_with_i32(
                    location + 2,
                    3,
                    GL::FLOAT,
                    false,
                    stride,
                    offset + 24,
                );
            }
            GlValueType::Mat4 => {
                gl.vertex_attrib_pointer_with_i32(location, 4, GL::FLOAT, false, stride, offset);
                gl.vertex_attrib_pointer_with_i32(
                    location + 1,
                    4,
                    GL::FLOAT,
                    false,
                    stride,
                    offset + 16,
                );
                gl.vertex_attrib_pointer_with_i32(
                    location + 2,
                    4,
                    GL::FLOAT,
                    false,
                    stride,
                    offset + 32,
                );
                gl.vertex_attrib_pointer_with_i32(
                    location + 3,
                    4,
                    GL::FLOAT,
                    false,
                    stride,
                    offset + 48,
                );
            }
        }
    }
}

#[derive(Clone)]
pub struct GlTexture {
    gl: GlContext,
    texture: Rc<WebGlTexture>,
}

impl GlTexture {
    pub fn new(
        gl: &GlContext,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        pixels: &[u8],
    ) -> GlTexture {
        Self::create(gl, width, height, pixel_format, Some(pixels))
    }

    pub fn empty(gl: &GlContext, width: u32, height: u32, pixel_format: PixelFormat) -> GlTexture {
        Self::create(gl, width, height, pixel_format, None)
    }

    fn create(
        gl: &GlContext,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        pixels: Option<&[u8]>,
    ) -> GlTexture {
        let gl = gl.clone();
        let texture = gl.create_texture().expect("Create Texture");

        let buf = pixels
            .as_ref()
            .map(|p| &p[0..width as usize * height as usize * pixel_format.byte_count()]);

        gl.bind_texture(GL::TEXTURE_2D, Some(&texture));

        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D,
            0,
            pixel_format.internal_format() as i32,
            width as i32,
            height as i32,
            0,
            pixel_format.format(),
            pixel_format._type(),
            buf,
        )
        .expect("Assign Texture");
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);

        GlTexture {
            gl,
            texture: Rc::new(texture),
        }
    }

    pub fn with_min_filter(self, filter: TextureFilter) -> GlTexture {
        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        self.gl
            .tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, filter.into());
        self
    }

    pub fn with_mag_filter(self, filter: TextureFilter) -> GlTexture {
        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        self.gl
            .tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, filter.into());
        self
    }

    pub fn sub_image(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        pixels: &[u8],
    ) {
        let buf = &pixels[0..width as usize * height as usize * pixel_format.byte_count()];

        self.gl.bind_texture(GL::TEXTURE_2D, Some(&self.texture));
        self.gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                pixel_format.internal_format(),
                pixel_format._type(),
                Some(buf),
            )
            .expect("Write to texture");
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

impl From<TextureFilter> for i32 {
    fn from(value: TextureFilter) -> Self {
        match value {
            TextureFilter::Nearest => GL::NEAREST as i32,
            TextureFilter::Linear => GL::LINEAR as i32,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PixelFormat {
    Alpha,
    RGB,
    RGBA,
    SRGB,
    SRGBA,
    U16,
}

impl PixelFormat {
    fn byte_count(&self) -> usize {
        match self {
            PixelFormat::Alpha => 1,
            PixelFormat::RGB | PixelFormat::SRGB => 3,
            PixelFormat::RGBA | PixelFormat::SRGBA => 4,
            PixelFormat::U16 => 2,
        }
    }

    // See: https://registry.khronos.org/webgl/specs/latest/2.0/#TEXTURE_TYPES_FORMATS_FROM_DOM_ELEMENTS_TABLE
    fn internal_format(&self) -> u32 {
        match self {
            PixelFormat::Alpha => GL::ALPHA,
            PixelFormat::RGB | PixelFormat::SRGB => GL::RGB,
            PixelFormat::RGBA | PixelFormat::SRGBA => GL::RGBA,
            PixelFormat::U16 => GL::RG8UI,
        }
    }

    fn format(&self) -> u32 {
        match self {
            PixelFormat::Alpha => GL::ALPHA,
            PixelFormat::RGB | PixelFormat::SRGB => GL::RGB,
            PixelFormat::RGBA | PixelFormat::SRGBA => GL::RGBA,
            PixelFormat::U16 => GL::RG_INTEGER,
        }
    }

    fn _type(&self) -> u32 {
        match self {
            PixelFormat::Alpha
            | PixelFormat::RGB
            | PixelFormat::SRGB
            | PixelFormat::RGBA
            | PixelFormat::SRGBA => GL::UNSIGNED_BYTE,
            PixelFormat::U16 => GL::UNSIGNED_BYTE,
        }
    }
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        if Rc::strong_count(&self.texture) == 1 {
            self.gl.delete_texture(Some(&self.texture));
        }
    }
}

pub struct GlFrameBuffer {
    gl: GlContext,
    texture: GlTexture,
    frame_buffer: WebGlFramebuffer,
    width: u32,
    height: u32,
}

impl GlFrameBuffer {
    pub fn new(gl: &GlContext, width: u32, height: u32) -> GlFrameBuffer {
        let gl = gl.clone();
        let texture = GlTexture::empty(&gl, width, height, PixelFormat::RGB);
        let frame_buffer = gl.create_framebuffer().expect("Create FrameBuffer");
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&frame_buffer));
        gl.framebuffer_texture_2d(
            GL::FRAMEBUFFER,
            GL::COLOR_ATTACHMENT0,
            GL::TEXTURE_2D,
            Some(&texture.texture),
            0,
        );
        gl.bind_framebuffer(GL::FRAMEBUFFER, None);

        Self {
            frame_buffer,
            texture,
            width,
            height,
            gl,
        }
    }

    pub fn bind(&self) {
        self.gl
            .viewport(0, 0, self.width as i32, self.height as i32);
        self.gl
            .bind_framebuffer(GL::FRAMEBUFFER, Some(&self.frame_buffer));
    }

    pub fn unbind(&self) {
        self.gl.bind_framebuffer(GL::FRAMEBUFFER, None);
    }

    pub fn texture(&self) -> &GlTexture {
        &self.texture
    }
}

impl Drop for GlFrameBuffer {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(Some(&self.frame_buffer));
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = OESElementIndexUint)]
    #[derive(Clone)]
    pub type OESElementIndexUint;
}
