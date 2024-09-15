use winit::keyboard::KeyCode;
use gilrs::Button;

use nes::Controller;

use std::collections::HashMap;

pub struct InputMap {
    map: HashMap<InputType, bool>,
}

impl InputMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn is_pressed(&self, key: impl Into<InputType>) -> bool {
        self.map.get(&key.into()).cloned().unwrap_or(false)
    }

    pub fn press(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = true)
            .or_insert(true);
    }

    pub fn release(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = false)
            .or_insert(false);
    }

    pub fn controller(&self) -> Controller {
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

    pub fn power(&self) -> bool {
        self.is_pressed(KeyCode::Delete)
    }

    pub fn reset(&self) -> bool {
        self.is_pressed(KeyCode::Backspace)
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum InputType {
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
