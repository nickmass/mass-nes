use crate::egui;

pub struct MemoryViewer<'a> {
    title: egui::WidgetText,
    mem: &'a [u8],
}

impl<'a> MemoryViewer<'a> {
    pub fn new(title: impl Into<egui::WidgetText>, mem: &'a [u8]) -> Self {
        Self {
            title: title.into(),
            mem,
        }
    }

    pub fn show(self, ctx: &egui::Context) {
        const CHUNK_SIZE: usize = 16;
        egui::Window::new(self.title).show(ctx, |ui| {
            let style = egui::TextStyle::Monospace;
            let line_height = ui.text_style_height(&style);
            ui.style_mut().override_text_style = Some(style);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::ScrollArea::vertical().show_rows(
                ui,
                line_height,
                self.mem.len() / CHUNK_SIZE,
                |ui, range| {
                    use std::fmt::Write;
                    for i in range {
                        let base_addr = i * CHUNK_SIZE;
                        let range = base_addr..base_addr + CHUNK_SIZE;
                        let chunk = &self.mem[range];

                        let mut line = String::with_capacity(80);
                        let _ = write!(line, "0x{:04X}:", base_addr);
                        for b in chunk {
                            let _ = write!(line, " {:02X}", b);
                        }
                        let _ = write!(line, "\t");
                        for b in chunk {
                            if b.is_ascii() && !b.is_ascii_control() {
                                let _ = write!(line, "{}", *b as char);
                            } else {
                                let _ = write!(line, ".");
                            }
                        }
                        ui.label(line);
                    }
                },
            );
        });
    }
}
