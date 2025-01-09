use std::sync::mpsc::{SendError, Sender, SyncSender};

pub use gilrs;
use gilrs::{EventType, Gilrs};
use winit::event::ElementState;
use winit::event_loop::{EventLoopClosed, EventLoopProxy};

pub struct GilrsInput<P: GamepadChannel> {
    gilrs: Gilrs,
    proxy: P,
    shutdown: bool,
}

impl<P: GamepadChannel> GilrsInput<P> {
    pub fn new(proxy: P) -> Result<Self, gilrs::Error> {
        let gilrs = Gilrs::new()?;
        Ok(Self {
            proxy,
            gilrs,
            shutdown: false,
        })
    }

    pub fn poll(&mut self) {
        while let Some(ev) = self.gilrs.next_event() {
            let event = match ev.event {
                EventType::ButtonPressed(button, _) => GamepadEvent::Button {
                    gamepad_id: ev.id,
                    state: ElementState::Pressed,
                    button,
                },
                EventType::ButtonReleased(button, _) => GamepadEvent::Button {
                    gamepad_id: ev.id,
                    state: ElementState::Released,
                    button,
                },
                EventType::AxisChanged(axis, value, _) => GamepadEvent::Axis {
                    gamepad_id: ev.id,
                    axis,
                    value,
                },
                EventType::Connected => GamepadEvent::Connected { gamepad_id: ev.id },
                EventType::Disconnected => GamepadEvent::Disconnected { gamepad_id: ev.id },
                _ => return,
            };

            if let Err(_) = self.proxy.send_event(event.into()) {
                self.shutdown = true;
            }
        }
    }
}

pub trait GamepadChannel {
    type Event: From<GamepadEvent> + Send + 'static;
    type Err;

    fn send_event(&self, event: Self::Event) -> Result<(), Self::Err>;
}

impl<E: From<GamepadEvent> + Send + 'static> GamepadChannel for EventLoopProxy<E> {
    type Event = E;

    type Err = EventLoopClosed<E>;

    fn send_event(&self, event: Self::Event) -> Result<(), Self::Err> {
        self.send_event(event)
    }
}

impl<E: From<GamepadEvent> + Send + 'static> GamepadChannel for SyncSender<E> {
    type Event = E;

    type Err = SendError<E>;

    fn send_event(&self, event: Self::Event) -> Result<(), Self::Err> {
        self.send(event)
    }
}

impl<E: From<GamepadEvent> + Send + 'static> GamepadChannel for Sender<E> {
    type Event = E;

    type Err = SendError<E>;

    fn send_event(&self, event: Self::Event) -> Result<(), Self::Err> {
        self.send(event)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum GamepadEvent {
    Button {
        gamepad_id: gilrs::GamepadId,
        state: ElementState,
        button: gilrs::Button,
    },
    Axis {
        gamepad_id: gilrs::GamepadId,
        axis: gilrs::Axis,
        value: f32,
    },
    Connected {
        gamepad_id: gilrs::GamepadId,
    },
    Disconnected {
        gamepad_id: gilrs::GamepadId,
    },
}
