use std::sync::mpsc::{Receiver, Sender, channel};

use futures::StreamExt;
use web_sys::HtmlCanvasElement;
use web_sys::wasm_bindgen::JsError;
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

#[derive(Debug)]
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
}

impl From<GamepadEvent> for UserEvent {
    fn from(value: GamepadEvent) -> Self {
        UserEvent::Gamepad(value)
    }
}

pub struct App<A> {
    audio: A,
    gamepad: Option<GilrsInput<EventLoopProxy<UserEvent>>>,
    canvas: HtmlCanvasElement,
    window: Option<Window>,
    event_loop: Option<EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<Sender<EmulatorInput>>,
    gfx_worker: GfxWorker,
    pause: bool,
}

impl<A: Audio + 'static> App<A> {
    pub fn new(
        gfx_worker: GfxWorker,
        audio: A,
        canvas: HtmlCanvasElement,
    ) -> Result<Self, JsError> {
        let event_loop = EventLoop::with_user_event().build()?;

        // Need to poll gamepad
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
            pause: false,
        })
    }

    pub fn proxy(&self) -> EventLoopProxy<UserEvent> {
        let Some(event_loop) = self.event_loop.as_ref() else {
            panic!("no event loop created");
        };

        event_loop.create_proxy()
    }

    pub fn nes_io(&mut self) -> NesInputs {
        let (tx, rx) = channel();

        self.input_tx = Some(tx);

        NesInputs { rx }
    }

    pub fn run(mut self) {
        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        if let Some(mut gamepad) = self.gamepad.take() {
            let gamepad_poll = async move {
                let mut stream = gloo::timers::future::IntervalStream::new(1);
                while let Some(_) = stream.next().await {
                    gamepad.poll();
                }
            };

            wasm_bindgen_futures::spawn_local(gamepad_poll);
        }

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

    fn send_inputs(&self) {
        if let Some(tx) = self.input_tx.as_ref() {
            if self.input.reset() {
                let _ = tx.send(UserInput::Reset.into());
            }

            if self.input.power() {
                let _ = tx.send(UserInput::Power.into());
            }

            let p1 = self.input.controller();
            let _ = tx.send(UserInput::PlayerOne(p1).into());
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

                    if self.input.pause() {
                        self.pause = !self.pause;
                        if self.pause {
                            self.audio.pause();
                        } else {
                            self.audio.play();
                        }
                    }
                    self.send_inputs();
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
            UserEvent::Gamepad(ev) => {
                match ev {
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
                }
                self.send_inputs();
            }
            UserEvent::Load(rom) => {
                if let Some(tx) = self.input_tx.as_mut() {
                    let _ = tx.send(EmulatorInput::Load(rom));
                    self.audio.play();
                }
            }
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
    rx: Receiver<EmulatorInput>,
}

impl NesInputs {
    pub fn try_recv(&mut self) -> impl Iterator<Item = EmulatorInput> + '_ {
        self.rx.try_iter()
    }
}
