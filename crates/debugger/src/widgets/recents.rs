use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use crate::egui;

pub struct Recents {
    limit: usize,
    files: VecDeque<PathBuf>,
}

impl Recents {
    pub fn new(recents: &[PathBuf], limit: usize) -> Self {
        let files = recents.into_iter().cloned().collect();

        Self { limit, files }
    }

    pub fn add(&mut self, file: PathBuf) {
        let Ok(file) = file.canonicalize() else {
            return;
        };

        self.files.retain(|f| f.as_path() != file.as_path());

        self.files.push_front(file);

        if self.files.len() > self.limit {
            self.files.pop_back();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Path> + '_ {
        self.files.iter().map(|p| p.as_path())
    }

    pub fn ui(&self, ui: &mut egui::Ui) -> Option<&Path> {
        let mut ret = None;
        if self.files.is_empty() {
            ui.add_enabled(false, egui::Label::new("Recent Files"));
        } else {
            ui.menu_button("Recent Files", |ui| {
                for f in self.iter() {
                    if let Some(name) = f.file_name().and_then(|s| s.to_str()) {
                        if ui.button(name).clicked() {
                            ret = Some(f);
                        }
                    }
                }
            });
        }

        ret
    }
}
