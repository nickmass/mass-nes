use std::time::Duration;

use eframe::egui;

use crate::platform::Timestamp;

#[derive(Debug, Copy, Clone)]
pub enum Message {
    Pause,
    Reset,
    Power,
    Rewind,
    FastForward,
    StepForward,
    StepBack,
    SaveState(u8),
    RestoreState(u8),
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Pause => write!(f, "Paused"),
            Message::Reset => write!(f, "Reset"),
            Message::Power => write!(f, "Power"),
            Message::Rewind => write!(f, "Rewind"),
            Message::FastForward => write!(f, "Fast Forward"),
            Message::StepForward => write!(f, "Step Forward"),
            Message::StepBack => write!(f, "Step Backward"),
            Message::SaveState(slot) => write!(f, "Saved state to slot #{}", slot + 1),
            Message::RestoreState(slot) => write!(f, "Restored state from slot #{}", slot + 1),
        }
    }
}

pub struct PopupMessage {
    message: Option<String>,
    creation_time: Timestamp,
}

impl PopupMessage {
    pub fn new() -> Self {
        Self {
            message: None,
            creation_time: Timestamp::now(),
        }
    }

    pub fn set_message(&mut self, message: Message) {
        self.message = Some(message.to_string());
        self.creation_time = Timestamp::now();
    }

    pub fn has_message(&self) -> bool {
        self.message.is_some()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if Timestamp::now().duration_since(self.creation_time) >= Duration::from_millis(500) {
            self.message = None;
            return;
        }

        let Some(msg) = self.message.as_ref() else {
            return;
        };

        let txt = egui::RichText::new(&*msg).strong().size(30.0);
        ui.add(egui::Label::new(txt).extend().selectable(false));
    }
}
