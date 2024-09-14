use gilrs::{EventType, Gilrs};
use winit::event::ElementState;
use winit::event_loop::EventLoopProxy;

pub struct GilrsInput<E: 'static> {
    gilrs: Gilrs,
    proxy: EventLoopProxy<E>,
    shutdown: bool,
}

impl<E: From<GamepadEvent> + Send + 'static> GilrsInput<E> {
    pub fn new(proxy: EventLoopProxy<E>) -> Result<Self, gilrs::Error> {
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
