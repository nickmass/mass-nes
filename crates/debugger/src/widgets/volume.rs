use crate::egui;

use egui::Widget;

pub enum VolumeChanged {
    Mute,
    Volume(f32),
}

pub struct VolumePicker<'a> {
    mute: &'a mut bool,
    volume: &'a mut f32,
}

impl<'a> VolumePicker<'a> {
    pub fn new(mute: &'a mut bool, volume: &'a mut f32) -> Self {
        Self { mute, volume }
    }

    pub fn ui(self, ui: &mut egui::Ui) -> Option<VolumeChanged> {
        let mut mute = *self.mute;
        let mut volume = *self.volume;
        if !mute && ui.button("🔊").clicked() {
            mute = true;
        } else if mute && ui.button("🔇").clicked() {
            mute = false;
        }
        if !mute {
            egui::Slider::new(&mut volume, 0.0..=1.0)
                .show_value(false)
                .ui(ui);
        }

        let ret = if mute && !*self.mute {
            Some(VolumeChanged::Mute)
        } else if !mute && *self.mute {
            Some(VolumeChanged::Volume(volume))
        } else if !mute && volume != *self.volume {
            Some(VolumeChanged::Volume(volume))
        } else {
            None
        };
        *self.mute = mute;
        *self.volume = volume;

        ret
    }
}
