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
            select: self.is_pressed(KeyCode::ShiftRight)
                || self.is_pressed(Button::Select) | self.is_pressed(KeyCode::Backslash),
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

#[cfg(feature = "egui")]
impl From<egui::Key> for InputType {
    fn from(value: egui::Key) -> Self {
        use egui::Key;
        let k = match value {
            Key::ArrowDown => KeyCode::ArrowDown,
            Key::ArrowUp => KeyCode::ArrowUp,
            Key::ArrowLeft => KeyCode::ArrowLeft,
            Key::ArrowRight => KeyCode::ArrowRight,
            Key::Escape => KeyCode::Escape,
            Key::Tab => KeyCode::Tab,
            Key::Backspace => KeyCode::Backspace,
            Key::Enter => KeyCode::Enter,
            Key::Space => KeyCode::Space,
            Key::Insert => KeyCode::Insert,
            Key::Delete => KeyCode::Delete,
            Key::Home => KeyCode::Home,
            Key::End => KeyCode::End,
            Key::PageUp => KeyCode::PageUp,
            Key::PageDown => KeyCode::PageDown,
            Key::Copy => KeyCode::Copy,
            Key::Cut => KeyCode::Cut,
            Key::Paste => KeyCode::Paste,
            Key::Comma => KeyCode::Comma,
            Key::Backslash => KeyCode::Backslash,
            Key::Slash => KeyCode::Slash,
            Key::Minus => KeyCode::Minus,
            Key::Period => KeyCode::Period,
            Key::Semicolon => KeyCode::Semicolon,
            Key::Quote => KeyCode::Quote,
            Key::Num0 => KeyCode::Digit0,
            Key::Num1 => KeyCode::Digit1,
            Key::Num2 => KeyCode::Digit2,
            Key::Num3 => KeyCode::Digit3,
            Key::Num4 => KeyCode::Digit4,
            Key::Num5 => KeyCode::Digit5,
            Key::Num6 => KeyCode::Digit6,
            Key::Num7 => KeyCode::Digit7,
            Key::Num8 => KeyCode::Digit8,
            Key::Num9 => KeyCode::Digit9,
            Key::A => KeyCode::KeyA,
            Key::B => KeyCode::KeyB,
            Key::C => KeyCode::KeyC,
            Key::D => KeyCode::KeyD,
            Key::E => KeyCode::KeyE,
            Key::F => KeyCode::KeyF,
            Key::G => KeyCode::KeyG,
            Key::H => KeyCode::KeyH,
            Key::I => KeyCode::KeyI,
            Key::J => KeyCode::KeyJ,
            Key::K => KeyCode::KeyK,
            Key::L => KeyCode::KeyL,
            Key::M => KeyCode::KeyM,
            Key::N => KeyCode::KeyN,
            Key::O => KeyCode::KeyO,
            Key::P => KeyCode::KeyP,
            Key::Q => KeyCode::KeyQ,
            Key::R => KeyCode::KeyR,
            Key::S => KeyCode::KeyS,
            Key::T => KeyCode::KeyT,
            Key::U => KeyCode::KeyU,
            Key::V => KeyCode::KeyV,
            Key::W => KeyCode::KeyW,
            Key::X => KeyCode::KeyX,
            Key::Y => KeyCode::KeyY,
            Key::Z => KeyCode::KeyZ,
            Key::F1 => KeyCode::F1,
            Key::F2 => KeyCode::F2,
            Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4,
            Key::F5 => KeyCode::F5,
            Key::F6 => KeyCode::F6,
            Key::F7 => KeyCode::F7,
            Key::F8 => KeyCode::F8,
            Key::F9 => KeyCode::F9,
            Key::F10 => KeyCode::F10,
            Key::F11 => KeyCode::F11,
            Key::F12 => KeyCode::F12,
            Key::F13 => KeyCode::F13,
            Key::F14 => KeyCode::F14,
            Key::F15 => KeyCode::F15,
            Key::F16 => KeyCode::F16,
            Key::F17 => KeyCode::F17,
            Key::F18 => KeyCode::F18,
            Key::F19 => KeyCode::F19,
            Key::F20 => KeyCode::F20,
            Key::F21 => KeyCode::F21,
            Key::F22 => KeyCode::F22,
            Key::F23 => KeyCode::F23,
            Key::F24 => KeyCode::F24,
            Key::F25 => KeyCode::F25,
            Key::F26 => KeyCode::F26,
            Key::F27 => KeyCode::F27,
            Key::F28 => KeyCode::F28,
            Key::F29 => KeyCode::F29,
            Key::F30 => KeyCode::F30,
            Key::F31 => KeyCode::F31,
            Key::F32 => KeyCode::F32,
            Key::F33 => KeyCode::F33,
            Key::F34 => KeyCode::F34,
            Key::F35 => KeyCode::F35,
            _ => KeyCode::F35,
        };
        InputType::Key(k)
    }
}
