use crate::{egui, runner::StepKind};
use eframe::egui::Vec2;
use tracing::instrument;

use crate::{
    cpu_6502::{Instruction, InstructionIter},
    debug_state::DebugUiState,
};

#[derive(Debug, Clone)]
pub struct Breakpoints {
    breakpoints: Vec<u16>,
}

impl Breakpoints {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
        }
    }

    pub fn toggle(&mut self, addr: u16) {
        if let Some(idx) = self.breakpoints.iter().position(|a| *a == addr) {
            self.breakpoints.remove(idx);
        } else {
            self.breakpoints.push(addr);
        }
    }

    pub fn is_set(&self, addr: u16) -> bool {
        self.breakpoints.iter().any(|a| *a == addr)
    }
}

struct InstructionUi(Instruction);

impl InstructionUi {
    fn pc(&self) -> u16 {
        self.0.pc()
    }

    fn inst_tooltip(&self, ui: &mut egui::Ui) {
        let op_code = &self.0.op_code;
        ui.style_mut().override_text_style = None;
        if op_code.illegal {
            ui.label(format!("{}\tUnofficial", op_code.instruction.mnemonic()));
        } else {
            ui.label(op_code.instruction.mnemonic());
        }
        ui.label(op_code.instruction.description());
        if op_code.dummy_cycles {
            ui.label(format!("Cycles: {}+", op_code.cycles));
        } else {
            ui.label(format!("Cycles: {}", op_code.cycles));
        }
    }

    fn ui(&self, reg_pc: u16, breakpoints: &mut Breakpoints, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let op_code = &self.0.op_code;
        ui.horizontal(|row_ui| {
            let mut set = breakpoints.is_set(self.0.pc());
            if BreakpointToggle::ui(&mut set, row_ui).changed() {
                breakpoints.toggle(self.0.pc());
                changed = true;
            }
            let bg_color = if self.pc() == reg_pc {
                egui::Color32::DARK_RED
            } else {
                egui::Color32::TRANSPARENT
            };

            let mut frame = egui::Frame::new().fill(bg_color).begin(row_ui);
            let ui = &mut frame.content_ui;
            ui.label(format!(
                "0x{:04X}: {:02X} {}",
                self.0.pc(),
                op_code.op_code,
                self.0.addressing.display_operands()
            ));

            let color = if op_code.illegal {
                egui::Color32::RED
            } else {
                ui.visuals().text_color()
            };

            let name = egui::RichText::new(op_code.instruction.mnemonic())
                .color(color)
                .strong();

            ui.label(name).on_hover_ui(|ui| self.inst_tooltip(ui));
            let addressing = format!("{}", self.0.addressing);
            ui.label(format!("{:40}", addressing));

            frame.end(row_ui);
        });

        changed
    }
}

pub struct BreakpointToggle;

impl BreakpointToggle {
    pub fn ui(value: &mut bool, ui: &mut egui::Ui) -> egui::Response {
        let mut frame = egui::Frame::new().begin(ui);
        let height = ui.text_style_height(&egui::TextStyle::Monospace);
        let size = egui::Vec2::splat(height);

        let (rect, mut res) = frame
            .content_ui
            .allocate_exact_size(size, egui::Sense::click());

        if *value {
            let origin = rect.min + (size / 2.0);
            frame
                .content_ui
                .painter()
                .circle_filled(origin, height / 2.0, egui::Color32::DARK_RED);
        }

        if res.clicked() {
            res.mark_changed();
            *value = !*value;
        }
        if res.hovered() {
            ui.output_mut(|platform| platform.cursor_icon = egui::CursorIcon::PointingHand);
        }

        frame.end(ui);
        res
    }
}

pub struct CodeViewer {
    offset: f32,
    jump_addr: String,
    step_kind: StepKind,
}

#[derive(Debug, Copy, Clone)]
pub enum CodeViewerAction {
    UpdateBreakpoint,
    Step(StepKind),
}

impl CodeViewer {
    pub fn new() -> Self {
        Self {
            offset: 0.0,
            jump_addr: String::new(),
            step_kind: StepKind::Instruction,
        }
    }

    #[instrument(skip_all)]
    pub fn show(
        &mut self,
        pause: &mut bool,
        debug: &DebugUiState,
        breakpoints: &mut Breakpoints,
        ctx: &egui::Context,
    ) -> Option<CodeViewerAction> {
        let mut breakpoint_changed = false;
        let mut step_requested = false;
        let mem = debug.cpu_mem();
        let state = debug.state();
        let reg_pc = state.cpu.instruction_addr.unwrap_or(state.cpu.reg_pc);
        egui::Window::new("Code").show(ctx, |ui| {
            let style = egui::TextStyle::Monospace;
            let line_height = ui.text_style_height(&style);
            let mut v_offset = None;

            ui.horizontal(|ui| {
                let play_pause = if *pause { "▶" } else { "⏸" };
                if ui.button(play_pause).clicked() {
                    *pause = !*pause;
                }
                self.jump_addr.retain(|c| c.is_ascii_hexdigit());
                let mut jump_addr = None;
                if ui.button("Jump to PC").clicked() {
                    jump_addr = Some(reg_pc);
                }
                if ui.button("Jump to...").clicked() {
                    jump_addr = Some(u16::from_str_radix(&self.jump_addr, 16).unwrap_or(0));
                }
                egui::TextEdit::singleline(&mut self.jump_addr)
                    .hint_text("0000")
                    .char_limit(4)
                    .desired_width(120.0)
                    .show(ui);

                if let Some(jump_addr) = jump_addr {
                    let offset = jump_addr as f32 * (line_height + ui.spacing().item_spacing.y);
                    v_offset = Some(offset)
                }

                if ui.button("Step").clicked() {
                    *pause = true;
                    step_requested = true;
                }
                egui::ComboBox::from_id_salt("step_combo")
                    .selected_text(self.step_kind.to_string())
                    .show_ui(ui, |ui| {
                        for &kind in StepKind::all() {
                            ui.selectable_value(&mut self.step_kind, kind, kind.to_string());
                        }
                    });
            });
            ui.separator();
            ui.style_mut().override_text_style = Some(style);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

            let scroll = egui::ScrollArea::vertical()
                .vertical_scroll_offset(v_offset.unwrap_or(self.offset))
                .show_rows(ui, line_height, 0x10000, |ui, range| {
                    let pc = range.start as u16;
                    for (_, inst) in range.zip(InstructionIter::new(|addr| mem[addr as usize], pc))
                    {
                        if inst.pc() >= pc {
                            breakpoint_changed |= InstructionUi(inst).ui(reg_pc, breakpoints, ui);
                        }
                    }
                    ui.allocate_space(Vec2::new(ui.available_width(), 0.0));
                });

            self.offset = scroll.state.offset.y;
        });

        if breakpoint_changed {
            Some(CodeViewerAction::UpdateBreakpoint)
        } else if step_requested {
            Some(CodeViewerAction::Step(self.step_kind))
        } else {
            None
        }
    }
}
