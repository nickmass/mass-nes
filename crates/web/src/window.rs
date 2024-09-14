use futures::Stream;
use gilrs::{Button};
use web_sys::{HtmlCanvasElement};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowAttributes;
use winit::platform::web::{WindowAttributesExtWebSys, EventLoopExtWebSys};

use std::collections::HashMap;

use crate::gl;

use super::audio::Audio;
use super::gamepad::{GamepadEvent, GilrsInput};
use super::gfx::Filter;
use super::sync::FrameSync;

use nes::{Controller, UserInput};

pub enum EmulatorInput {
    UserInput(UserInput),
    Load(Vec<u8>)
}

impl From<UserInput> for EmulatorInput {
    fn from(value: UserInput) -> Self {
        EmulatorInput::UserInput(value)
    }
}

pub enum UserEvent {
    Frame(Frame),
    Samples(Samples),
    Gamepad(GamepadEvent),
    Load(Vec<u8>),
    Sync,
}

impl From<GamepadEvent> for UserEvent {
    fn from(value: GamepadEvent) -> Self {
        UserEvent::Gamepad(value)
    }
}


pub struct Frame(Vec<u16>);

pub struct Samples(Vec<i16>);

pub struct App<F, A, S> {
    audio: A,
    sync: Option<S>,
    gfx: Gfx<F>,
    gamepad: Option<GilrsInput<UserEvent>>,
    window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<futures::channel::mpsc::Sender<EmulatorInput>>,
}

impl<F: Filter + 'static, A: Audio + 'static, S: FrameSync + 'static> App<F, A, S> {
    pub fn new(filter: F, audio: A, sync: S, canvas: HtmlCanvasElement) -> Self {
        let event_loop = winit::event_loop::EventLoop::with_user_event()
            .build()
            .unwrap();

        let window = event_loop.create_window(WindowAttributes::default()
                                              .with_prevent_default(false)
                                              .with_canvas(Some(canvas.clone()))).unwrap();


        let gfx = Gfx::new(canvas, filter);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        let gamepad = GilrsInput::new(event_loop.create_proxy()).unwrap();

        Self {
            audio,
            sync: Some(sync),
            window,
            gfx,
            event_loop: Some(event_loop),
            input: InputMap::new(),
            input_tx: None,
            gamepad: Some(gamepad),
        }
    }

    pub fn proxy(&self) -> winit::event_loop::EventLoopProxy<UserEvent> {
        let Some(event_loop) = self.event_loop.as_ref() else {
            panic!("no event loop created");
        };

        event_loop.create_proxy()
    }

    pub fn nes_io(&mut self) -> (NesInputs, NesOutputs) {
        let output = NesOutputs {
            proxy: self.proxy(),
        };

        let (tx, rx) = futures::channel::mpsc::channel(10);

        self.input_tx = Some(tx);

        let input = NesInputs { rx };

        (input, output)
    }

    pub fn run(mut self) {
        if let Some(sync) = self.sync.take() {
            let sync_proxy = self.proxy();
            wasm_bindgen_futures::spawn_local(sync_loop(sync, sync_proxy));
        } else {
            panic!("no frame sync provided");
        }

        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        event_loop.spawn_app(self);
    }
}

async fn sync_loop<S: FrameSync>(mut sync: S, proxy: EventLoopProxy<UserEvent>) {
    loop {
        sync.sync_frame().await;
        let _ = proxy.send_event(UserEvent::Sync);
    }
}

impl<F: Filter, A: Audio, S: FrameSync> winit::application::ApplicationHandler<UserEvent>
    for App<F, A, S>
{
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::Resized(size) => {
                self.gfx.resize(size.into());
                self.window.request_redraw();
            }
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state.is_pressed() {
                        self.input.press(key);
                    } else {
                        self.input.release(key);
                    }
                }
            }
            winit::event::WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                self.window.request_redraw();
            }
            winit::event::WindowEvent::RedrawRequested => {
                if self.window.is_visible() != Some(false) {
                    self.gfx.render();
                }

                if let Some(gamepad) = self.gamepad.as_mut() {
                    gamepad.poll();
                }
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Frame(frame) => {
                self.gfx.update_frame(frame);
                self.window.request_redraw();
            }
            UserEvent::Samples(Samples(samples)) => self.audio.add_samples(samples),
            UserEvent::Sync => {
                if let Some(tx) = self.input_tx.as_mut() {
                    let p1 = self.input.controller();

                    if self.input.reset() {
                        let _ = tx.try_send(UserInput::Reset.into());
                    }

                    if self.input.power() {
                        let _ = tx.try_send(UserInput::Power.into());
                    }

                    let _ = tx.try_send(UserInput::PlayerOne(p1).into());
                }
            }
            UserEvent::Gamepad(ev) => match ev {
                GamepadEvent::Button {
                    gamepad_id: _,
                    state,
                    button,
                } => {
                    if state.is_pressed() {
                        self.input.press(button);
                    } else {
                        self.input.release(button);
                    }
                }
                _ => (),
            },
            UserEvent::Load(rom) => {
                if let Some(tx) = self.input_tx.as_mut() {
                    let _ = tx.try_send(EmulatorInput::Load(rom));
                    self.gfx.focus();
                    self.audio.play();
                }
            }
        }
    }
}

struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2]
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

struct Gfx<T> {
    filter: T,
    canvas: HtmlCanvasElement,
    gl: gl::GlContext,
    screen: gl::GlModel<Vertex>,
    program: gl::GlProgram,
    size: (f64, f64),
    frame: Option<Vec<u16>>,
}

impl<T: Filter> Gfx<T> {
    fn new(canvas: HtmlCanvasElement, filter: T) -> Self {
        let (width, height) = filter.dimensions();
        let size = (width as f64, height as f64);

        let gl = gl::GlContext::new(canvas.clone());

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
            frame: None,
        }
    }

    pub fn resize(&mut self, (c_width, c_height): (u32, u32)) {
        let (width, height) = self.filter.dimensions();
        let (f_width, f_height) = (width as f64, height as f64);
        let (c_width, c_height) = (c_width as f64, c_height as f64);

        if (f_width - self.size.0).abs() < 5.0 && (f_height - self.size.1).abs() < 5.0 {
            return;
        }

        let (width, height) = if f_width < f_height {
            let ratio = f_height / f_width;
            (c_width, c_width * ratio)
        } else {
            let ratio = f_width / f_height;
            (c_height * ratio, c_height)
        };


        self.canvas.set_width(width as u32);
        self.canvas.set_height(height as u32);
        self.size = (width, height);
    }

    pub fn update_frame(&mut self, Frame(frame): Frame) {
        self.frame = Some(frame);
    }

    pub fn focus(&self) {
        let _ = self.canvas.focus();
    }

    pub fn render(&mut self) {
        let Some(screen) = self.frame.as_ref() else {
            return;
        };

        let uniforms = self.filter.process(&self.gl, self.size, screen.as_ref());

        let (width, height) = (self.size.0 as i32, self.size.1 as i32);
        self.gl.viewport(0, 0, width, height);
        self.program.draw(&self.screen, &uniforms, None);
        self.gl.flush();
    }
}

pub struct NesInputs {
    rx: futures::channel::mpsc::Receiver<EmulatorInput>,
}

impl NesInputs {
    pub fn inputs(self) -> impl Stream<Item = EmulatorInput> {
        self.rx
    }
}

pub struct NesOutputs {
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
}

impl NesOutputs {
    pub fn send_frame(&self, frame: Vec<u16>) {
        let _ = self.proxy.send_event(UserEvent::Frame(Frame(frame)));
    }

    pub fn send_samples(&self, samples: Vec<i16>) {
        let _ = self.proxy.send_event(UserEvent::Samples(Samples(samples)));
    }
}

struct InputMap {
    map: HashMap<InputType, bool>,
}

impl InputMap {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn is_pressed(&self, key: impl Into<InputType>) -> bool {
        self.map.get(&key.into()).cloned().unwrap_or(false)
    }

    fn press(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = true)
            .or_insert(true);
    }

    fn release(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = false)
            .or_insert(false);
    }

    fn controller(&self) -> Controller {
        Controller {
            a: self.is_pressed(KeyCode::KeyZ)
                || self.is_pressed(Button::East)
                || self.is_pressed(Button::West),
            b: self.is_pressed(KeyCode::KeyX) || self.is_pressed(Button::South),
            select: self.is_pressed(KeyCode::ShiftRight) || self.is_pressed(Button::Select),
            start: self.is_pressed(KeyCode::Enter) || self.is_pressed(Button::Start),
            up: self.is_pressed(KeyCode::ArrowUp) || self.is_pressed(Button::DPadUp),
            down: self.is_pressed(KeyCode::ArrowDown) || self.is_pressed(Button::DPadDown),
            left: self.is_pressed(KeyCode::ArrowLeft) || self.is_pressed(Button::DPadLeft),
            right: self.is_pressed(KeyCode::ArrowRight) || self.is_pressed(Button::DPadRight),
        }
    }

    fn power(&self) -> bool {
        self.is_pressed(KeyCode::Delete)
    }

    fn reset(&self) -> bool {
        self.is_pressed(KeyCode::Backspace)
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum InputType {
    Key(KeyCode),
    Button(Button),
}

impl From<KeyCode> for InputType {
    fn from(value: KeyCode) -> Self {
        InputType::Key(value)
    }
}

impl From<Button> for InputType {
    fn from(value: Button) -> Self {
        InputType::Button(value)
    }
}
