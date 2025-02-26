pub use platform::Help;

#[cfg(not(target_arch = "wasm32"))]
use desktop as platform;
#[cfg(target_arch = "wasm32")]
use web as platform;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use crate::app::{AppEventsProxy, EmulatorControl};
    pub struct Help;

    impl Help {
        pub fn new(_proxy: AppEventsProxy, _control: EmulatorControl) -> Self {
            Self
        }

        pub fn show(&mut self, _ctx: &eframe::egui::Context) {}
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use gloo::net::http;
    use nes::Region;
    use serde::Deserialize;
    use std::{
        path::PathBuf,
        sync::mpsc::{Receiver, Sender, channel},
    };

    use eframe::egui::{self, Response, Ui};

    use crate::app::{AppEvent, AppEventsProxy, EmulatorControl};

    #[derive(Debug, Clone, Deserialize)]
    pub struct Rom {
        title: String,
        author: String,
        link: String,
        url: String,
    }

    impl Rom {
        fn download(&self, proxy: AppEventsProxy, control: EmulatorControl) {
            let url = self.url.clone();
            let download = async move {
                if let Ok(res) = http::RequestBuilder::new(&url)
                    .method(http::Method::GET)
                    .send()
                    .await
                {
                    if let Ok(bytes) = res.binary().await {
                        control.load_rom(Region::Ntsc, bytes, "example.nes".to_string(), None);
                        proxy.send(AppEvent::RomLoaded(PathBuf::new()));
                    }
                }
            };

            wasm_bindgen_futures::spawn_local(download);
        }

        fn show(&self, ui: &mut Ui) -> Response {
            ui.label(&self.title);
            ui.horizontal(|ui| {
                ui.label("by:");
                if ui.link(&self.author).clicked() {
                    eframe::web::open_url(&self.link, true);
                }
            });
            ui.button("Load")
        }
    }

    pub struct Help {
        proxy: AppEventsProxy,
        control: EmulatorControl,
        roms: Vec<Rom>,
        rx: Receiver<Rom>,
    }

    impl Help {
        pub fn new(proxy: AppEventsProxy, control: EmulatorControl) -> Self {
            let (tx, rx) = channel();

            populate_roms(tx);

            Self {
                proxy,
                control,
                roms: Vec::new(),
                rx,
            }
        }

        pub fn show(&mut self, ctx: &egui::Context) {
            egui::Window::new("Help").show(ctx, |ui| {
                for rom in self.rx.try_iter() {
                    self.roms.push(rom);
                }

                ui.heading("Source Code");
                if ui.link("https://github.com/nickmass/mass-nes/").clicked() {
                    eframe::web::open_url("https://github.com/nickmass/mass-nes/", true);
                }

                ui.separator();
                ui.heading("Controls");
                egui::Grid::new("help_controls").show(ui, |ui| {
                    const CONTROLS: &[(&str, &str)] = &[
                        ("Dpad", "Arrow Keys"),
                        ("A", "Z"),
                        ("B", "X"),
                        ("Select", "Backslash"),
                        ("Start", "Enter"),
                        ("Reset", "Backspace"),
                        ("Power", "Delete"),
                        ("Pause", "Space"),
                        ("Rewind", "Tab"),
                    ];

                    for &(nes, pc) in CONTROLS {
                        ui.label(nes);
                        ui.label(pc);
                        ui.end_row();
                    }
                });

                ui.separator();
                ui.heading("Example Roms");
                egui::Grid::new("help_roms").show(ui, |ui| {
                    for rom in &self.roms {
                        if rom.show(ui).clicked() {
                            rom.download(self.proxy.clone(), self.control.clone());
                        }
                        ui.end_row();
                    }
                });
            });
        }
    }

    fn populate_roms(tx: Sender<Rom>) {
        let get_list = async move {
            if let Ok(res) = http::RequestBuilder::new("roms/romlist.json")
                .method(http::Method::GET)
                .send()
                .await
            {
                if let Ok(bytes) = res.binary().await {
                    let roms: Vec<_> = serde_json::from_slice(&bytes).unwrap();
                    for rom in roms {
                        let _ = tx.send(rom);
                    }
                }
            }
        };

        wasm_bindgen_futures::spawn_local(get_list);
    }
}
