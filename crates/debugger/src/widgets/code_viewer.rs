use crate::egui;
use egui::RichText;
use tracing::instrument;

use crate::cpu_6502::{Instruction, InstructionIter};

struct InstructionUi(Instruction);

impl InstructionUi {
    fn ui(&self, ui: &mut egui::Ui) {
        let op_code = &self.0.op_code;
        ui.horizontal(|ui| {
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

            let name = RichText::new(op_code.instruction.mnemonic())
                .color(color)
                .strong();

            ui.label(name).on_hover_ui(|ui| {
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
            });
            let addressing = format!("{}", self.0.addressing);
            ui.label(format!("{:40}", addressing));
        });
    }
}

pub struct CodeViewer<'a> {
    mem: &'a [u8],
}

impl<'a> CodeViewer<'a> {
    pub fn new(mem: &'a [u8]) -> Self {
        Self { mem }
    }

    #[instrument(skip_all)]
    pub fn show(self, ctx: &egui::Context) {
        egui::Window::new("Code").show(ctx, |ui| {
            let style = egui::TextStyle::Monospace;
            let line_height = ui.text_style_height(&style);
            ui.style_mut().override_text_style = Some(style);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::ScrollArea::vertical().show_rows(ui, line_height, 0x10000, |ui, range| {
                let pc = range.start as u16;
                for (_, inst) in range.zip(InstructionIter::new(|addr| self.mem[addr as usize], pc))
                {
                    if inst.pc() >= pc {
                        InstructionUi(inst).ui(ui);
                    }
                }
            });
        });
    }
}
