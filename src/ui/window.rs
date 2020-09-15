use glium::glutin::event::Event;
use glium::glutin::event::VirtualKeyCode as Key;
use glium::glutin::event::WindowEvent;
use glium::glutin::event_loop::{EventLoop, EventLoopProxy};
use glium::implement_vertex;
use glium::Display;
use glium::Surface;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::gfx::Filter;

use nes::{Controller, UserInput};

enum UserEvent {
    Frame(Frame),
}

struct Frame(Vec<u16>);

pub struct WindowHandle {
    proxy: EventLoopProxy<UserEvent>,
    input: Arc<Mutex<HashMap<Key, bool>>>,
    closed: Arc<AtomicBool>,
}

impl WindowHandle {
    pub fn send_frame(&self, frame: Vec<u16>) {
        let _ = self.proxy.send_event(UserEvent::Frame(Frame(frame)));
    }

    pub fn input(&self) -> Vec<UserInput> {
        let mut r = Vec::new();
        let input = self.input.lock().unwrap();
        let p1 = Controller {
            a: *input.get(&Key::Z).unwrap_or(&false),
            b: *input.get(&Key::X).unwrap_or(&false),
            select: *input.get(&Key::RShift).unwrap_or(&false),
            start: *input.get(&Key::Return).unwrap_or(&false),
            up: *input.get(&Key::Up).unwrap_or(&false),
            down: *input.get(&Key::Down).unwrap_or(&false),
            left: *input.get(&Key::Left).unwrap_or(&false),
            right: *input.get(&Key::Right).unwrap_or(&false),
        };

        if *input.get(&Key::Delete).unwrap_or(&false) {
            r.push(UserInput::Power);
        }

        if *input.get(&Key::Back).unwrap_or(&false) {
            r.push(UserInput::Reset);
        }

        r.push(UserInput::PlayerOne(p1));
        r
    }

    pub fn closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct Window<T: Filter + 'static> {
    filter: T,
    event_loop: Option<EventLoop<UserEvent>>,
    proxy: EventLoopProxy<UserEvent>,
    display: Display,
    indicies: glium::index::NoIndices,
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    closed: Arc<AtomicBool>,
    input: Arc<Mutex<HashMap<Key, bool>>>,
    size: (f64, f64),
}

impl<T: Filter> Window<T> {
    pub fn new(filter: T) -> Window<T> {
        let dims = filter.get_dimensions();
        let event_loop = glium::glutin::event_loop::EventLoop::with_user_event();
        let window = glium::glutin::window::WindowBuilder::new()
            .with_inner_size(glium::glutin::dpi::PhysicalSize::new(dims.0, dims.1))
            .with_title("Mass NES");

        let context = glium::glutin::ContextBuilder::new();

        let display = glium::Display::new(window, context, &event_loop)
            .expect("Could not initialize display");

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
        )
        .unwrap();

        let closed = Arc::new(AtomicBool::new(false));
        let input = Arc::new(Mutex::new(HashMap::new()));
        let proxy = event_loop.create_proxy();

        Window {
            filter,
            event_loop: Some(event_loop),
            proxy,
            display,
            indicies,
            program,
            vertex_buffer,
            closed,
            input,
            size: (dims.0 as f64, dims.1 as f64),
        }
    }

    pub fn handle(&self) -> WindowHandle {
        WindowHandle {
            proxy: self.proxy.clone(),
            input: self.input.clone(),
            closed: self.closed.clone(),
        }
    }

    pub fn run(mut self) {
        let mut frame = None;
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _window_id, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    self.size = new_size.into();
                }
                WindowEvent::CloseRequested => {
                    self.closed.store(true, Ordering::SeqCst);
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = input.state == glium::glutin::event::ElementState::Pressed;
                    if let Some(key) = input.virtual_keycode {
                        let mut input = self.input.lock().unwrap();
                        input.insert(key, pressed);
                    }
                }
                _ => (),
            },
            Event::UserEvent(event) => match event {
                UserEvent::Frame(next_frame) => {
                    frame = Some(next_frame.0);
                    self.display.gl_window().window().request_redraw();
                }
            },
            Event::MainEventsCleared => {
                *control_flow = glium::glutin::event_loop::ControlFlow::Wait;
            }
            Event::RedrawRequested(_) => {
                if let Some(frame) = &frame {
                    self.render(frame);
                }
            }
            _ => (),
        });
    }

    pub fn render(&self, screen: &[u16]) {
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
