use gilrs::Button;
use winit::keyboard::KeyCode;

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

    pub fn save_state(&self) -> Option<u8> {
        const MAP: &[(KeyCode, u8)] = &[
            (KeyCode::Digit1, 0),
            (KeyCode::Digit2, 1),
            (KeyCode::Digit3, 2),
            (KeyCode::Digit4, 3),
            (KeyCode::Digit5, 4),
            (KeyCode::Digit6, 5),
            (KeyCode::Digit7, 6),
            (KeyCode::Digit8, 7),
            (KeyCode::Digit9, 8),
            (KeyCode::Digit0, 9),
        ];

        for &(key, slot) in MAP.iter() {
            if self.is_pressed(key) {
                return Some(slot);
            }
        }

        None
    }

    pub fn restore_state(&self) -> Option<u8> {
        const MAP: &[(KeyCode, u8)] = &[
            (KeyCode::F1, 0),
            (KeyCode::F2, 1),
            (KeyCode::F3, 2),
            (KeyCode::F4, 3),
            (KeyCode::F5, 4),
            (KeyCode::F6, 5),
            (KeyCode::F7, 6),
            (KeyCode::F8, 7),
            (KeyCode::F9, 8),
            (KeyCode::F10, 9),
        ];

        for &(key, slot) in MAP.iter() {
            if self.is_pressed(key) {
                return Some(slot);
            }
        }

        None
    }

    pub fn rewind(&self) -> bool {
        self.is_pressed(KeyCode::Tab) | self.is_pressed(Button::LeftTrigger)
    }

    pub fn pause(&self) -> bool {
        self.is_pressed(KeyCode::Space)
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
