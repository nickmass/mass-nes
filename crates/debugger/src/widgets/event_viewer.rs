use eframe::egui;
use egui::{Color32, Context, Pos2, Rect, Ui, Vec2, Widget, ecolor::Hsva};
use nes::DebugEvent;
use serde::{Deserialize, Serialize};

use crate::debug_state::DebugUiState;

const KNOWN_EVENTS: &[(&str, DebugEvent)] = &[
    ("PPUCTL (2000) W", DebugEvent::CpuWrite(0x2000)),
    ("PPUMASK (2001) W", DebugEvent::CpuWrite(0x2001)),
    ("PPUSTATUS (2002) R", DebugEvent::CpuRead(0x2002)),
    ("OAMADDR (2003) W", DebugEvent::CpuWrite(0x2003)),
    ("OAMDATA (2004) R", DebugEvent::CpuRead(0x2004)),
    ("OAMDATA (2004) W", DebugEvent::CpuWrite(0x2004)),
    ("PPUSCROLL (2005) W", DebugEvent::CpuWrite(0x2005)),
    ("PPUADDR (2006) W", DebugEvent::CpuWrite(0x2006)),
    ("PPUDATA (2007) R", DebugEvent::CpuRead(0x2007)),
    ("PPUDATA (2007) W", DebugEvent::CpuWrite(0x2007)),
    ("OAMDMA (4014) W", DebugEvent::CpuWrite(0x4014)),
    ("Sprite Zero", DebugEvent::SpriteZero),
    ("Sprite Overflow", DebugEvent::SpriteOverflow),
    ("Fetch Nametable", DebugEvent::FetchNt),
    ("Fetch Attribute", DebugEvent::FetchAttr),
    ("Fetch Background", DebugEvent::FetchBg),
    ("Fetch Sprite", DebugEvent::FetchSprite),
    ("Mapper IRQ", DebugEvent::MapperIrq),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Interest {
    event: DebugEvent,
    breakpoint: bool,
    log: bool,
    color: Color32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Interests {
    interests: Vec<Interest>,
}

impl Interests {
    pub fn new() -> Self {
        Self {
            interests: Vec::new(),
        }
    }

    fn push(&mut self, event: DebugEvent) {
        if self.interests.len() == 16 {
            return;
        }

        let mut color = Color32::WHITE;
        for n in 0..16 {
            let h = ((n * 7) as f32 / 16.0).fract();
            let [r, g, b] = Hsva::new(h, 1.0, 1.0, 1.0).to_srgb();
            color = Color32::from_rgb(r, g, b);

            if self.interests.iter().all(|i| i.color != color) {
                break;
            }
        }

        let interest = Interest {
            event,
            color,
            log: false,
            breakpoint: false,
        };
        self.interests.push(interest);
    }

    fn remove(&mut self, idx: usize) {
        if idx < self.interests.len() {
            self.interests.remove(idx);
        }
    }

    fn contains(&self, event: &DebugEvent) -> bool {
        self.interests.iter().any(|i| i.event == *event)
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Interest> + '_ {
        self.interests.iter_mut()
    }

    pub fn events(&self) -> impl Iterator<Item = DebugEvent> + '_ {
        self.interests.iter().map(|i| i.event)
    }

    pub fn breakpoint_mask(&self) -> u16 {
        let mut v = 0;
        let mut n = 1;
        for i in self.interests.iter() {
            if i.breakpoint {
                v |= n;
            }
            n <<= 1;
        }

        v
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Device {
    Cpu,
    Ppu,
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Device::Cpu => write!(f, "CPU"),
            Device::Ppu => write!(f, "PPU"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Access {
    Read,
    Write,
    Execute,
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Access::Read => write!(f, "Read"),
            Access::Write => write!(f, "Write"),
            Access::Execute => write!(f, "Execute"),
        }
    }
}

struct DisplayEvent(DebugEvent, Option<u8>);

impl std::fmt::Display for DisplayEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DisplayEvent(ev, data) = *self;
        for &(txt, event) in KNOWN_EVENTS.iter() {
            if ev == event {
                return match (ev, data) {
                    (
                        DebugEvent::CpuRead(_) | DebugEvent::CpuWrite(_) | DebugEvent::CpuExec(_),
                        Some(d),
                    ) => write!(f, "{txt} = {d:02X}"),
                    _ => write!(f, "{txt}"),
                };
            }
        }

        match (ev, data) {
            (DebugEvent::CpuRead(a), Some(d)) => write!(f, "CPU Read {a:04X} = {d:02X}"),
            (DebugEvent::CpuWrite(a), Some(d)) => write!(f, "CPU Write {a:04X} = {d:02X}"),
            (DebugEvent::CpuExec(a), Some(d)) => write!(f, "CPU Exec {a:04X} = {d:02X}"),
            (DebugEvent::CpuRead(a), _) => write!(f, "CPU Read {a:04X}"),
            (DebugEvent::CpuWrite(a), _) => write!(f, "CPU Write {a:04X}"),
            (DebugEvent::CpuExec(a), _) => write!(f, "CPU Exec {a:04X}"),
            (DebugEvent::PpuRead(a), _) => write!(f, "PPU Read {a:04X}"),
            (DebugEvent::PpuWrite(a), _) => write!(f, "PPU Write {a:04X}"),
            (DebugEvent::SpriteZero, _) => write!(f, "Sprite Zero"),
            (DebugEvent::SpriteOverflow, _) => write!(f, "Sprite Overflow"),
            (DebugEvent::FetchNt, _) => write!(f, "Fetch Nametable"),
            (DebugEvent::FetchAttr, _) => write!(f, "Fetch Attribute"),
            (DebugEvent::FetchBg, _) => write!(f, "Fetch Background"),
            (DebugEvent::FetchSprite, _) => write!(f, "Fetch Sprite"),
            (DebugEvent::MapperIrq, _) => write!(f, "Mapper IRQ"),
            (DebugEvent::Dot(s, d), _) => write!(f, "Dot {s}x{d}"),
        }
    }
}

struct EventEntry {
    event: DebugEvent,
    color: Color32,
    scanline: u16,
    dot: u16,
    data: u8,
}

impl EventEntry {
    fn ui(&self, ui: &mut Ui) -> egui::Response {
        let res = ui.horizontal(|ui| {
            let height = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
            let (rect, res) = ui.allocate_exact_size(
                Vec2::new(ui.style().spacing.item_spacing.x, height),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(rect, 0.0, self.color);
            let label_res = ui.label(format!("{} x {}", self.scanline, self.dot));

            res.union(label_res)
        });
        let label_res = ui.label(format!("{}", DisplayEvent(self.event, Some(self.data))));

        res.inner.union(label_res)
    }
}

pub struct EventViewer {
    texture: Option<egui::TextureHandle>,
    event_log: Vec<EventEntry>,
    age: u64,
    pixel_buf: Vec<u8>,
    add_device: Device,
    add_access: Access,
    add_address: String,
    highlight: Option<(u16, u16, Color32)>,
}

impl EventViewer {
    pub fn new() -> Self {
        Self {
            texture: None,
            event_log: Vec::new(),
            age: 0,
            pixel_buf: vec![0; 312 * 341 * 3],
            add_device: Device::Cpu,
            add_access: Access::Read,
            add_address: String::new(),
            highlight: None,
        }
    }

    fn render_events(&mut self, debug: &DebugUiState, ctx: &Context, interests: &Interests) {
        for scanline in 0..312 {
            for dot in 0..341 {
                let frame_idx = scanline * 256 + dot;
                let event_idx = scanline * 341 + dot;

                let scanline = scanline as u16;
                let dot = dot as u16;

                let pal_id = if scanline < 240 && dot < 256 {
                    debug.frame()[frame_idx]
                } else {
                    0
                };

                let (_data, mut events) = debug.events()[event_idx];
                let (r, g, b) = if events == 0 {
                    if let Some((hi_line, hi_dot, hi_color)) = self.highlight.clone() {
                        if hi_line.abs_diff(scanline) <= 1 && hi_dot.abs_diff(dot) <= 1 {
                            let (r, g, b, _) = hi_color.to_tuple();
                            (r, g, b)
                        } else {
                            debug.palette().lookup(pal_id)
                        }
                    } else {
                        debug.palette().lookup(pal_id)
                    }
                } else {
                    let mut color = Color32::BLACK;
                    let mut n = 0;
                    while events != 0 {
                        if events & 1 == 1 {
                            events >>= 1;

                            if events == 0 {
                                color = interests
                                    .interests
                                    .get(n)
                                    .map(|i| i.color)
                                    .unwrap_or_default();
                            } else {
                                color = Color32::WHITE;
                            }

                            break;
                        } else {
                            events >>= 1;
                            n += 1;
                        }
                    }
                    let (r, g, b, _) = color.to_tuple();
                    (r, g, b)
                };

                let idx = event_idx * 3;

                self.pixel_buf[idx] = r;
                self.pixel_buf[idx + 1] = g;
                self.pixel_buf[idx + 2] = b;
            }
        }

        let image = egui::ColorImage::from_rgb([341, 312], &self.pixel_buf);

        self.texture = Some(ctx.load_texture("events", image, egui::TextureOptions::NEAREST));
    }

    fn populate_log(&mut self, debug: &DebugUiState, interests: &Interests) {
        self.event_log.clear();

        for scanline in 0..312 {
            for dot in 0..341 {
                let event_idx = scanline * 341 + dot;
                let scanline = scanline as u16;
                let dot = dot as u16;

                let (data, mut events) = debug.events()[event_idx];
                let mut n = 0;
                while events != 0 {
                    if events & 1 == 1 {
                        if let Some(interest) = interests.interests.get(n) {
                            if interest.log {
                                let entry = EventEntry {
                                    data,
                                    event: interest.event,
                                    color: interest.color,
                                    scanline,
                                    dot,
                                };
                                self.event_log.push(entry);
                            }
                        }
                    }
                    events >>= 1;
                    n += 1;
                }
            }
        }
    }

    fn is_expired(&self, now: u64, debug_interval: u64) -> bool {
        if let Some(_) = &self.texture {
            now - self.age >= debug_interval
        } else {
            true
        }
    }

    fn interest_picker(&mut self, ui: &mut Ui, interests: &mut Interests) -> bool {
        let mut changed = false;
        ui.vertical(|ui| {
            egui::Grid::new("interest_picker").show(ui, |ui| {
                let mut to_remove = None;
                for (idx, interest) in interests.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        changed |=
                            super::BreakpointToggle::ui(&mut interest.breakpoint, ui).changed();
                        egui::color_picker::color_edit_button_srgba(
                            ui,
                            &mut interest.color,
                            egui::color_picker::Alpha::Opaque,
                        );
                        ui.checkbox(&mut interest.log, "");
                        ui.label(format!("{}", DisplayEvent(interest.event, None)));
                    });
                    if ui.small_button("❌").clicked() {
                        to_remove = Some(idx);
                    }
                    ui.end_row();
                }

                if let Some(to_remove) = to_remove {
                    interests.remove(to_remove);
                    changed = true;
                }
            });

            if interests.events().any(|_| true) {
                ui.separator();
            }

            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("add_interest_device")
                    .selected_text(format!("{}", self.add_device))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.add_device, Device::Cpu, "CPU");
                        ui.selectable_value(&mut self.add_device, Device::Ppu, "PPU");
                    });

                if self.add_device == Device::Ppu && self.add_access == Access::Execute {
                    self.add_access = Access::Read;
                }

                egui::ComboBox::from_id_salt("add_interest_access")
                    .selected_text(format!("{}", self.add_access))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.add_access, Access::Read, "Read");
                        ui.selectable_value(&mut self.add_access, Access::Write, "Write");
                        if self.add_device != Device::Ppu {
                            ui.selectable_value(&mut self.add_access, Access::Execute, "Execute");
                        }
                    });
            });

            ui.horizontal(|ui| {
                self.add_address.retain(|c| c.is_ascii_hexdigit());
                egui::TextEdit::singleline(&mut self.add_address)
                    .hint_text("0000")
                    .desired_width(80.0)
                    .char_limit(4)
                    .show(ui);

                if ui.small_button("✅").clicked() {
                    if let Some(address) = u16::from_str_radix(&self.add_address, 16).ok() {
                        let event = match (self.add_device, self.add_access) {
                            (Device::Cpu, Access::Read) => DebugEvent::CpuRead(address),
                            (Device::Cpu, Access::Write) => DebugEvent::CpuWrite(address),
                            (Device::Cpu, Access::Execute) => DebugEvent::CpuExec(address),
                            (Device::Ppu, Access::Read) => DebugEvent::PpuRead(address),
                            (Device::Ppu, Access::Write) => DebugEvent::PpuWrite(address),
                            (Device::Ppu, Access::Execute) => DebugEvent::PpuRead(address),
                        };

                        interests.push(event);
                        changed = true;
                    }
                }
            });

            ui.separator();

            egui::Grid::new("known_interests").show(ui, |ui| {
                for (name, event) in KNOWN_EVENTS.iter() {
                    if interests.contains(event) {
                        continue;
                    }
                    ui.label(*name);
                    if ui.small_button("✅").clicked() {
                        interests.push(*event);
                        changed = true;
                    }
                    ui.end_row();
                }
            });
        });

        changed
    }

    pub fn show(
        &mut self,
        region: &crate::app::Region,
        debug: &DebugUiState,
        debug_interval: u64,
        interests: &mut Interests,
        ctx: &Context,
    ) -> bool {
        let mut changed = false;
        let now = debug.now();
        if self.is_expired(now, debug_interval) {
            self.render_events(debug, ctx, interests);
            self.populate_log(debug, interests);
            self.age = now;
        }

        let max_lines = match region {
            crate::app::Region::Ntsc => 262,
            crate::app::Region::Pal => 312,
        };

        self.highlight = None;
        egui::Window::new("Event Viewer")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.texture {
                        let mut res = egui::Image::new(tex)
                            .maintain_aspect_ratio(false)
                            .fit_to_exact_size(Vec2::new(341.0, max_lines as f32) * 3.0)
                            .uv(Rect::from_two_pos(
                                Pos2::ZERO,
                                (1.0, 1.0 / 312.0 * max_lines as f32).into(),
                            ))
                            .ui(ui);
                        if let Some(pos) = res.hover_pos() {
                            let pos = Pos2::new(pos.x - res.rect.left(), pos.y - res.rect.top());
                            let scanline = pos.y.floor() as usize / 3;
                            let dot = pos.x.floor() as usize / 3;
                            let idx = scanline * 341 + dot;

                            let (data, mut ev) = debug.events()[idx];
                            let mut n = 0;
                            if scanline > 261 {
                                res = res.on_hover_text_at_pointer(format!(
                                    "Scanline {scanline} x Dot {dot} PAL"
                                ));
                            } else {
                                res = res.on_hover_text_at_pointer(format!(
                                    "Scanline {scanline} x Dot {dot}"
                                ));
                            }
                            while ev != 0 {
                                if ev & 1 == 1 {
                                    if let Some(interest) = interests.interests.get(n) {
                                        let display = DisplayEvent(interest.event, Some(data));
                                        res = res.on_hover_text_at_pointer(format!("{display}"));
                                    }
                                }
                                ev >>= 1;
                                n += 1;
                            }
                        }
                    }

                    ui.vertical(|ui| {
                        changed |= self.interest_picker(ui, interests);

                        if !self.event_log.is_empty() {
                            ui.separator();

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                egui::Grid::new("event_viewer_log").show(ui, |ui| {
                                    for log in self.event_log.iter() {
                                        if log.ui(ui).hovered() {
                                            self.highlight =
                                                Some((log.scanline, log.dot, log.color));
                                            self.age = 0;
                                        }
                                        ui.end_row();
                                    }
                                });

                                ui.allocate_at_least(
                                    Vec2::new(ui.available_width(), 0.0),
                                    egui::Sense::empty(),
                                );
                            });
                        }
                    });
                });
            });

        changed
    }
}
