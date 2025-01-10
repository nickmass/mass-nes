use glium::glutin::config::ConfigTemplateBuilder;
use glium::winit;
use winit::keyboard::PhysicalKey;

use nes::UserInput;
use ui::audio::Audio;
use ui::filters::Filter;
use ui::gamepad::{GamepadEvent, GilrsInput};
use ui::input::InputMap;

use super::gfx::{Gfx, GfxBackBuffer};
use super::sync::FrameSync;

pub enum UserEvent {
    Frame,
    Gamepad(GamepadEvent),
    Sync,
}

impl From<GamepadEvent> for UserEvent {
    fn from(value: GamepadEvent) -> Self {
        UserEvent::Gamepad(value)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum EmulatorInput {
    Nes(UserInput),
    SaveState(u8),
    RestoreState(u8),
    Rewind,
}

impl From<UserInput> for EmulatorInput {
    fn from(value: UserInput) -> Self {
        EmulatorInput::Nes(value)
    }
}

pub struct App<F, A, S> {
    audio: A,
    sync: Option<S>,
    gfx: Gfx<F>,
    gamepad: Option<GilrsInput<winit::event_loop::EventLoopProxy<UserEvent>>>,
    window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<std::sync::mpsc::Sender<EmulatorInput>>,
    back_buffer: GfxBackBuffer,
    pause: bool,
}

impl<F: Filter, A: Audio, S: FrameSync> App<F, A, S> {
    pub fn new(filter: F, audio: A, sync: S) -> Self {
        let event_loop = winit::event_loop::EventLoop::with_user_event()
            .build()
            .unwrap();

        let dims = filter.dimensions();

        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_config_template_builder(
                ConfigTemplateBuilder::new().with_swap_interval(None, None),
            )
            .with_vsync(false)
            .with_inner_size(dims.0, dims.1)
            .with_title("Mass NES")
            .build(&event_loop);

        let proxy = event_loop.create_proxy();
        let back_buffer = GfxBackBuffer::new(proxy);
        let gfx = Gfx::new(display, back_buffer.clone(), filter);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        window.set_cursor_visible(false);

        let gamepad = GilrsInput::new(event_loop.create_proxy()).unwrap();

        Self {
            audio,
            sync: Some(sync),
            window,
            gfx,
            back_buffer,
            event_loop: Some(event_loop),
            input: InputMap::new(),
            input_tx: None,
            gamepad: Some(gamepad),
            pause: false,
        }
    }

    fn proxy(&self) -> winit::event_loop::EventLoopProxy<UserEvent> {
        let Some(event_loop) = self.event_loop.as_ref() else {
            panic!("no event loop created");
        };

        event_loop.create_proxy()
    }

    pub fn back_buffer(&self) -> GfxBackBuffer {
        self.back_buffer.clone()
    }

    pub fn nes_io(&mut self) -> NesInputs {
        let (tx, rx) = std::sync::mpsc::channel();

        self.input_tx = Some(tx);

        NesInputs { rx }
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

        if let Some(mut gamepad) = self.gamepad.take() {
            let _ = std::thread::Builder::new()
                .name("gamepad".into())
                .spawn(move || loop {
                    gamepad.poll();
                });
        }

        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        self.audio.play();

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

                    if self.input.pause() {
                        self.pause = !self.pause;
                        if self.pause {
                            self.audio.pause();
                        } else {
                            self.audio.play();
                        }
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
            UserEvent::Frame => {
                self.gfx.swap();
                self.window.request_redraw();
            }
            UserEvent::Sync => {
                if let Some(tx) = self.input_tx.as_ref() {
                    let p1 = self.input.controller();

                    if self.input.reset() {
                        let _ = tx.send(UserInput::Reset.into());
                    }

                    if self.input.power() {
                        let _ = tx.send(UserInput::Power.into());
                    }

                    if let Some(slot) = self.input.save_state() {
                        let _ = tx.send(EmulatorInput::SaveState(slot));
                    }

                    if let Some(slot) = self.input.restore_state() {
                        let _ = tx.send(EmulatorInput::RestoreState(slot));
                    }

                    if self.input.rewind() {
                        let _ = tx.send(EmulatorInput::Rewind);
                    }

                    let _ = tx.send(UserInput::PlayerOne(p1).into());
                }
            }
            UserEvent::Gamepad(ev) => match ev {
                GamepadEvent::Button { state, button, .. } => {
                    if state.is_pressed() {
                        self.input.press(button);
                    } else {
                        self.input.release(button);
                    }
                }
                GamepadEvent::Axis { axis, value, .. } => {
                    self.input.axis(axis, value);
                }
                _ => (),
            },
        }
    }
}

pub struct NesInputs {
    rx: std::sync::mpsc::Receiver<EmulatorInput>,
}

impl NesInputs {
    pub fn inputs(self) -> impl Iterator<Item = EmulatorInput> {
        self.rx.into_iter()
    }
}
