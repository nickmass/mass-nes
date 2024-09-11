use gilrs::{Axis, Button};
use glium::glutin::config::ConfigTemplateBuilder;
use glium::implement_vertex;
use glium::winit::keyboard::{KeyCode, PhysicalKey};
use glium::{glutin, winit};
use glium::{Display, Surface};

use std::collections::HashMap;

use super::audio::Audio;
use super::gamepad::{GamepadEvent, GilrsInput};
use super::gfx::Filter;
use super::sync::FrameSync;

use nes::{Controller, UserInput};

enum UserEvent {
    Frame(Frame),
    Samples(Samples),
    Gamepad(GamepadEvent),
    Sync,
}

impl From<GamepadEvent> for UserEvent {
    fn from(value: GamepadEvent) -> Self {
        UserEvent::Gamepad(value)
    }
}

struct Frame(Vec<u16>);

struct Samples(Vec<i16>);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct App<F, A, S> {
    audio: A,
    sync: Option<S>,
    gfx: Gfx<F>,
    gamepad: Option<GilrsInput<UserEvent>>,
    window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<std::sync::mpsc::Sender<UserInput>>,
}

impl<F: Filter, A: Audio, S: FrameSync> App<F, A, S> {
    pub fn new(filter: F, audio: A, sync: S) -> Self {
        let event_loop = winit::event_loop::EventLoop::with_user_event()
            .build()
            .unwrap();

        let dims = filter.get_dimensions();

        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_config_template_builder(
                ConfigTemplateBuilder::new().with_swap_interval(None, None),
            )
            .with_inner_size(dims.0, dims.1)
            .with_title("Mass NES")
            .build(&event_loop);

        let gfx = Gfx::new(display, filter);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        window.set_cursor_visible(false);

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

    fn proxy(&self) -> winit::event_loop::EventLoopProxy<UserEvent> {
        let Some(event_loop) = self.event_loop.as_ref() else {
            panic!("no event loop created");
        };

        event_loop.create_proxy()
    }

    pub fn nes_io(&mut self) -> (NesInputs, NesOutputs) {
        let output = NesOutputs {
            proxy: self.proxy(),
        };

        let (tx, rx) = std::sync::mpsc::channel();

        self.input_tx = Some(tx);

        let input = NesInputs { rx };

        (input, output)
    }

    pub fn run(mut self) -> ! {
        if let Some(mut sync) = self.sync.take() {
            let sync_proxy = self.proxy();
            std::thread::Builder::new()
                .name("sync".into())
                .spawn(move || loop {
                    sync.sync_frame();
                    let _ = sync_proxy.send_event(UserEvent::Sync);
                })
                .unwrap();
        } else {
            panic!("no frame sync provided");
        }

        if let Some(gamepad) = self.gamepad.take() {
            gamepad.run();
        }

        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        let Err(err) = event_loop.run_app(&mut self) else {
            std::process::exit(0)
        };

        panic!("{:?}", err)
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
                if let Some(tx) = self.input_tx.as_ref() {
                    let p1 = self.input.controller();

                    if self.input.reset() {
                        let _ = tx.send(UserInput::Reset);
                    }

                    if self.input.power() {
                        let _ = tx.send(UserInput::Power);
                    }

                    let _ = tx.send(UserInput::PlayerOne(p1));
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
        }
    }
}

struct Gfx<T> {
    filter: T,
    display: Display<glutin::surface::WindowSurface>,
    indicies: glium::index::NoIndices,
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    size: (f64, f64),
    frame: Option<Vec<u16>>,
}

impl<T: Filter> Gfx<T> {
    fn new(display: Display<glutin::surface::WindowSurface>, filter: T) -> Self {
        eprintln!(
            "OpenGL: ver: {}, glsl: {:?}, vendor: {}, renderer: {}",
            display.get_opengl_version_string(),
            display.get_supported_glsl_version(),
            display.get_opengl_vendor_string(),
            display.get_opengl_renderer_string()
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

        let shape = vec![top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let program = glium::Program::from_source(
            &display,
            &*filter.get_vertex_shader(),
            &*filter.get_fragment_shader(),
            None,
        );

        let program = match program {
            Ok(p) => p,
            Err(glium::CompilationError(msg, kind)) => {
                panic!("Shader Compilation Errror '{kind:?}':\n{msg}")
            }
            Err(e) => panic!("{e:?}"),
        };

        let size = filter.get_dimensions();
        let size = (size.0 as f64, size.1 as f64);

        Self {
            filter,
            display,
            indicies,
            program,
            vertex_buffer,
            size,
            frame: None,
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.display.resize(size);
        let size = (size.0 as f64, size.1 as f64);
        self.size = size;
    }

    pub fn update_frame(&mut self, Frame(frame): Frame) {
        self.frame = Some(frame);
    }

    pub fn render(&mut self) {
        let Some(screen) = self.frame.as_ref() else {
            return;
        };
        let uniforms = self.filter.process(&self.display, self.size, screen);
        let mut target = self.display.draw();

        let (filter_width, filter_height) = self.filter.get_dimensions();
        let (filter_width, filter_height) = (filter_width as f64, filter_height as f64);
        let (window_width, window_height) = self.size;
        let (surface_width, surface_height) = target.get_dimensions();
        let (surface_width, surface_height) = (surface_width as f64, surface_height as f64);
        let filter_ratio = filter_width / filter_height;
        let surface_ratio = surface_width / surface_height;

        let (left, bottom, width, height) = if filter_ratio > surface_ratio {
            let target_height = (1.0 / filter_ratio) * window_height;
            let target_height = (target_height / window_height) * surface_height * surface_ratio;
            (
                0,
                ((surface_height - target_height) / 2.0) as u32,
                surface_width as u32,
                target_height as u32,
            )
        } else {
            let target_width = (filter_ratio) * window_width;
            let target_width =
                (target_width / window_width) * surface_width * (1.0 / surface_ratio);
            (
                ((surface_width - target_width) / 2.0) as u32,
                0,
                target_width as u32,
                surface_height as u32,
            )
        };

        let params = glium::DrawParameters {
            viewport: Some(glium::Rect {
                left,
                bottom,
                width,
                height,
            }),
            ..Default::default()
        };

        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &self.vertex_buffer,
                &self.indicies,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
        target.finish().unwrap();
    }
}

pub struct NesInputs {
    rx: std::sync::mpsc::Receiver<UserInput>,
}

impl NesInputs {
    pub fn inputs(self) -> impl Iterator<Item = UserInput> {
        self.rx.into_iter()
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
