use futures::Stream;
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::PhysicalKey;
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowAttributes};

use super::gfx::Gfx;
use super::sync::FrameSync;

use nes::UserInput;
use ui::audio::Audio;
use ui::filters::Filter;
use ui::gamepad::{GamepadEvent, GilrsInput};
use ui::input::InputMap;

pub enum EmulatorInput {
    UserInput(UserInput),
    Load(Vec<u8>),
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
    window: Window,
    event_loop: Option<EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<futures::channel::mpsc::Sender<EmulatorInput>>,
}

impl<F: Filter + 'static, A: Audio + 'static, S: FrameSync + 'static> App<F, A, S> {
    pub fn new(filter: F, audio: A, sync: S, canvas: HtmlCanvasElement) -> Self {
        let event_loop = EventLoop::with_user_event().build().unwrap();

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_prevent_default(true)
                    .with_canvas(Some(canvas.clone())),
            )
            .unwrap();

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

    pub fn proxy(&self) -> EventLoopProxy<UserEvent> {
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

impl<F: Filter, A: Audio, S: FrameSync> ApplicationHandler<UserEvent> for App<F, A, S> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                self.gfx.resize(size.into());
                self.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
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
            WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if self.window.is_visible() != Some(false) {
                    self.gfx.render();
                }
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Frame(Frame(frame)) => {
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
                    self.audio.play();
                }
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        if let Some(gamepad) = self.gamepad.as_mut() {
            gamepad.poll();
        }
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
    proxy: EventLoopProxy<UserEvent>,
}

impl NesOutputs {
    pub fn send_frame(&self, frame: Vec<u16>) {
        let _ = self.proxy.send_event(UserEvent::Frame(Frame(frame)));
    }

    pub fn send_samples(&self, samples: Vec<i16>) {
        let _ = self.proxy.send_event(UserEvent::Samples(Samples(samples)));
    }
}
