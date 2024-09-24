use futures::Stream;
use web_sys::wasm_bindgen::JsError;
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::PhysicalKey;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowAttributes};

use crate::gfx::GfxRequest;
use crate::offscreen_gfx::GfxWorker;

use nes::UserInput;
use ui::audio::Audio;
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
    Gamepad(GamepadEvent),
    Load(Vec<u8>),
    Sync,
}

impl From<GamepadEvent> for UserEvent {
    fn from(value: GamepadEvent) -> Self {
        UserEvent::Gamepad(value)
    }
}

pub struct App<A> {
    audio: A,
    gamepad: Option<GilrsInput<UserEvent>>,
    canvas: HtmlCanvasElement,
    window: Option<Window>,
    event_loop: Option<EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<futures::channel::mpsc::Sender<EmulatorInput>>,
    gfx_worker: GfxWorker,
}

impl<A: Audio + 'static> App<A> {
    pub fn new(
        gfx_worker: GfxWorker,
        audio: A,
        canvas: HtmlCanvasElement,
    ) -> Result<Self, JsError> {
        let event_loop = EventLoop::with_user_event().build()?;

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        let gamepad = GilrsInput::new(event_loop.create_proxy())?;

        Ok(Self {
            audio,
            canvas,
            window: None,
            event_loop: Some(event_loop),
            input: InputMap::new(),
            input_tx: None,
            gamepad: Some(gamepad),
            gfx_worker,
        })
    }

    pub fn proxy(&self) -> EventLoopProxy<UserEvent> {
        let Some(event_loop) = self.event_loop.as_ref() else {
            panic!("no event loop created");
        };

        event_loop.create_proxy()
    }

    pub fn nes_io(&mut self) -> NesInputs {
        let (tx, rx) = futures::channel::mpsc::channel(10);

        self.input_tx = Some(tx);

        NesInputs { rx }
    }

    pub fn run(mut self) {
        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        self.run_loop(event_loop)
    }

    #[cfg(target_arch = "wasm32")]
    fn run_loop(self, event_loop: EventLoop<UserEvent>) {
        event_loop.spawn_app(self);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_loop(self, _event_loop: EventLoop<UserEvent>) {
        panic!("unsupported platform")
    }
}

impl<A> App<A> {
    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl<A: Audio> ApplicationHandler<UserEvent> for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        self.window = event_loop
            .create_window(window_attributes(self.canvas.clone()))
            .ok();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                let (x, y) = size.into();
                let _ = self.gfx_worker.tx.try_send(GfxRequest::Resize(x, y));
                self.request_redraw();
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
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let _ = self.gfx_worker.tx.try_send(GfxRequest::Redraw);
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
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

#[cfg(target_arch = "wasm32")]
fn window_attributes(canvas: HtmlCanvasElement) -> WindowAttributes {
    WindowAttributes::default()
        .with_prevent_default(true)
        .with_canvas(Some(canvas))
}

#[cfg(not(target_arch = "wasm32"))]
fn window_attributes(_canvas: HtmlCanvasElement) -> WindowAttributes {
    WindowAttributes::default()
}

pub struct NesInputs {
    rx: futures::channel::mpsc::Receiver<EmulatorInput>,
}

impl NesInputs {
    pub fn inputs(self) -> impl Stream<Item = EmulatorInput> {
        self.rx
    }
}