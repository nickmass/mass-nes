use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use crate::egui::{self, Color32};
use crate::platform::Timestamp;

#[derive(Clone)]
pub struct MessageStore {
    inner: Arc<Mutex<MessageStoreInner>>,
}

impl MessageStore {
    pub fn new(capacity: usize) -> Self {
        let inner = MessageStoreInner::new(capacity);

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, MessageStoreInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, MessageStoreInner>> {
        self.inner.try_lock().ok()
    }
}

pub struct MessageStoreInner {
    start: Timestamp,
    capacity: usize,
    messages: VecDeque<Message>,
}

impl MessageStoreInner {
    fn new(capacity: usize) -> Self {
        Self {
            start: Timestamp::now(),
            capacity,
            messages: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, message: Message) {
        if self.messages.len() >= self.capacity {
            self.messages.pop_back();
        }
        self.messages.push_front(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn get(&self, index: usize) -> Option<&'_ Message> {
        self.messages.get(index)
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub time: Timestamp,
    pub level: tracing::Level,
    pub message: String,
}

pub struct Messages {
    store: MessageStore,
}

impl Messages {
    pub fn new(store: MessageStore) -> Self {
        Self { store }
    }

    pub fn show(&self, ctx: &egui::Context) {
        egui::Window::new("Messages").show(ctx, |ui| {
            let style = egui::TextStyle::Monospace;
            let line_height = ui.text_style_height(&style);
            ui.style_mut().override_text_style = Some(style);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

            let store = self.store.lock();

            egui::ScrollArea::vertical().show_rows(ui, line_height, store.len(), |ui, range| {
                for msg in range.filter_map(|i| store.get(i)) {
                    ui.horizontal(|ui| {
                        let elapsed = DisplayDuration(msg.time.duration_since(store.start));
                        let level =
                            egui::RichText::new(msg.level.as_str()).color(level_color(msg.level));

                        ui.weak(format!("{}", elapsed));
                        ui.label(level);
                        ui.label(&msg.message);
                        ui.allocate_space(egui::Vec2::new(ui.available_width(), 0.0));
                    });
                }
            });
        });
    }
}

fn level_color(level: tracing::Level) -> Color32 {
    use tracing::Level;
    if level == Level::DEBUG {
        Color32::from_rgb(0x34, 0x65, 0xA4)
    } else if level == Level::TRACE {
        Color32::from_rgb(0x75, 0x50, 0x7B)
    } else if level == Level::INFO {
        Color32::from_rgb(0x4E, 0x9A, 0x06)
    } else if level == Level::WARN {
        Color32::from_rgb(0xC4, 0xA0, 0x00)
    } else if level == Level::ERROR {
        Color32::from_rgb(0xCC, 0x00, 0x00)
    } else {
        Color32::from_rgb(0x4E, 0x9A, 0x06)
    }
}

struct DisplayDuration(Duration);

impl std::fmt::Display for DisplayDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_secs = self.0.as_secs();
        let hours = total_secs / 3600;
        let mins = (total_secs / 60) % 60;
        let secs = total_secs % 60;
        let millis = self.0.subsec_millis();

        write!(f, "{hours:02}:{mins:02}:{secs:02}.{millis:03}")
    }
}

struct MessageWriter {
    level: tracing::Level,
    time: Timestamp,
    message: String,
}

impl MessageWriter {
    fn new(level: tracing::Level) -> Self {
        Self {
            level,
            time: Timestamp::now(),
            message: String::new(),
        }
    }
}

impl From<MessageWriter> for Message {
    fn from(
        MessageWriter {
            time,
            level,
            message,
        }: MessageWriter,
    ) -> Self {
        Message {
            time,
            level,
            message,
        }
    }
}

impl tracing::field::Visit for MessageWriter {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        let padding = if self.message.is_empty() { "" } else { ", " };

        let _ = match field.name() {
            "message" => write!(self.message, "{}{value:?}", padding),
            name => write!(self.message, "{}{}: {value:?}", padding, name),
        };
    }
}

pub struct EguiMessageLayer {
    store: MessageStore,
}

impl EguiMessageLayer {
    pub fn new(store: MessageStore) -> Self {
        Self { store }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for EguiMessageLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut writer = MessageWriter::new(event.metadata().level().clone());
        event.record(&mut writer);

        let Some(mut store) = self.store.try_lock() else {
            return;
        };

        store.push(writer.into());
    }
}
