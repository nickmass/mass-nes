pub use eframe::{egui, egui_glow};

use audio::{CpalSync, FrameSync};
use eframe::{
    egui::{Event, Widget},
    CreationContext,
};
use gfx::{Gfx, GfxBackBuffer};
use nes::UserInput;
use runner::{DebugRequest, EmulatorInput};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use ui::{
    audio::{Audio, CpalAudio, SamplesProducer},
    filters::{NesNtscSetup, PalettedFilter},
    input::{InputMap, InputType},
};

use std::{
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

mod audio;
mod debug_state;
mod gfx;
mod gl;
mod runner;
mod widgets;

use debug_state::{DebugSwapState, DebugUiState, Palette};
use widgets::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Region {
    Ntsc,
    Pal,
}

impl Into<nes::Region> for Region {
    fn into(self) -> nes::Region {
        match self {
            Region::Ntsc => nes::Region::Ntsc,
            Region::Pal => nes::Region::Pal,
        }
    }
}

fn main() {
    init_tracing();

    let options = eframe::NativeOptions {
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "Mass Emu",
        options,
        Box::new(|cc| Ok(Box::new(DebuggerApp::new(cc)?))),
    )
    .unwrap();
}

fn init_tracing() {
    use tracing::Level;
    use tracing_subscriber::{filter, layer::SubscriberExt, Layer};

    let tracy =
        tracing_tracy::TracyLayer::default().with_filter(filter::Targets::new().with_targets([
            ("debugger", Level::TRACE),
            ("nes", Level::TRACE),
            ("ui", Level::TRACE),
        ]));
    let log = tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::DEBUG);

    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(tracy).with(log))
        .expect("init tracing");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default = "Default::default")]
struct UiState {
    region: Region,
    volume: f32,
    mute: bool,
    show_screen: bool,
    focus_screen: bool,
    show_nametables: bool,
    show_cpu_mem: bool,
    show_ppu_mem: bool,
    show_chr_tiles: bool,
    auto_open_most_recent: bool,
    recent_files: Vec<PathBuf>,
    debug_interval: u64,
    selected_palette: u8,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            region: Region::Ntsc,
            volume: 1.0,
            mute: false,
            show_screen: true,
            focus_screen: false,
            show_nametables: false,
            show_cpu_mem: false,
            show_ppu_mem: false,
            show_chr_tiles: false,
            auto_open_most_recent: true,
            recent_files: Vec::new(),
            debug_interval: 10,
            selected_palette: 0,
        }
    }
}

struct DebuggerApp {
    app_events: AppEvents,
    input: SharedInput,
    emu_control: EmulatorControl,
    audio: CpalAudio<CpalSync>,
    nes_screen: NesScreen<PalettedFilter>,
    recents: Recents,
    last_input: InputState,
    pause: bool,
    debug: DebugUiState,
    state: UiState,
    chr_tiles: ChrTiles,
    nt_viewer: NametableViewer,
}

impl DebuggerApp {
    fn new(
        cc: &CreationContext,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let ntsc_setup = NesNtscSetup::composite();
        let filter = PalettedFilter::new(ntsc_setup.generate_palette());
        let palette = ntsc_setup.generate_palette();

        let gl = cc.gl.as_ref().expect("require glow opengl context").clone();
        let back_buffer = GfxBackBuffer::new(cc.egui_ctx.clone());
        let gfx = Gfx::new(gl, back_buffer.clone(), &palette, filter)?;
        let nes_screen = NesScreen::new(gfx);
        let palette = Palette::new(palette);

        let input = SharedInput::new();
        let last_input = input.state();
        let (emu_control, emu_commands) = EmulatorControl::new();
        let debug_swap = DebugSwapState::new();
        let debug = DebugUiState::new(debug_swap.clone(), palette);
        let chr_tiles = ChrTiles::new();
        let nt_viewer = NametableViewer::new();

        let (mut audio, sync, samples) =
            CpalAudio::new(CpalSync::new(), nes::Region::Ntsc.refresh_rate(), 64).unwrap();

        spawn_sync_thread(input.clone(), emu_control.clone(), sync);

        spawn_machine_thread(
            emu_commands,
            back_buffer,
            samples,
            audio.sample_rate(),
            debug_swap,
        );
        audio.pause();

        let state = if let Some(storage) = cc.storage {
            storage
                .get_string("debugger_ui_state")
                .and_then(|s| ron::from_str(&s).ok())
        } else {
            None
        };
        let state: UiState = state.unwrap_or_default();

        let app_events = AppEvents::new();

        let mut app = DebuggerApp {
            app_events,
            input,
            emu_control,
            audio,
            nes_screen,
            last_input,
            pause: false,
            state,
            debug,
            chr_tiles,
            nt_viewer,
            recents: Recents::new(&[], 10),
        };

        app.hydrate();

        Ok(app)
    }

    fn hydrate(&mut self) {
        if self.state.mute {
            self.set_volume(0.0);
        } else {
            self.set_volume(self.state.volume);
        }

        self.recents = Recents::new(&self.state.recent_files.as_slice(), 10);

        if self.state.auto_open_most_recent {
            if let Some(recent) = self.state.recent_files.first() {
                self.load_rom(recent.clone())
            }
        }

        self.update_debug_req();
    }

    fn select_rom(&self) {
        let control = self.emu_control.clone();
        let region = self.state.region;
        let proxy = self.app_events.create_proxy();
        std::thread::spawn(move || {
            let rom_file = rfd::FileDialog::new()
                .add_filter("NES Roms", &["nes"])
                .pick_file();

            if let Some((path, bytes)) = rom_file.and_then(|p| {
                let bytes = std::fs::read(&p).ok();
                Some(p).zip(bytes)
            }) {
                control.load_rom(region.into(), bytes);
                proxy.send(AppEvent::RomLoaded(path));
            }
        });
    }

    fn set_volume(&mut self, value: f32) {
        self.audio.volume(value);
    }

    fn handle_input(&mut self, input_state: InputState) {
        if !self.last_input.pause && input_state.pause {
            self.pause = !self.pause;
            if self.pause {
                self.audio.pause();
            } else {
                self.audio.play();
            }
        }

        self.last_input = input_state;
    }

    fn process_app_events(&mut self) {
        while let Some(ev) = self.app_events.poll_event() {
            self.handle_app_event(ev);
        }
    }

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::RomLoaded(path) => {
                self.recents.add(path);
                self.state.recent_files = self.recents.iter().map(|p| p.to_path_buf()).collect();
                self.audio.play();
                self.emu_control.sync();
            }
        }
    }

    fn load_rom(&self, rom_file: PathBuf) {
        if let Some(bytes) = std::fs::read(&rom_file).ok() {
            self.emu_control.load_rom(self.state.region.into(), bytes);
            self.app_events.send(AppEvent::RomLoaded(rom_file));
        }
    }

    fn update_debug_req(&self) {
        let mut debug = DebugRequest {
            interval: self.state.debug_interval,
            cpu_mem: self.state.show_cpu_mem | self.state.show_nametables,
            ppu_mem: self.state.show_ppu_mem
                | self.state.show_chr_tiles
                | self.state.show_nametables,
            pal_ram: self.state.show_chr_tiles | self.state.show_nametables,
            sprite_ram: false,
        };

        if !debug.cpu_mem && !debug.ppu_mem && !debug.pal_ram && !debug.sprite_ram {
            debug.interval = 0;
        }

        self.emu_control.debug_request(debug);
    }
}

impl eframe::App for DebuggerApp {
    #[instrument(skip_all)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_app_events();

        egui::TopBottomPanel::top("Menu Area").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.select_rom();
                        ui.close_menu();
                    }
                    if let Some(file) = self.recents.ui(ui) {
                        self.load_rom(file.to_path_buf());
                    }

                    ui.menu_button("Region", |ui| {
                        ui.radio_value(&mut self.state.region, Region::Ntsc, "NTSC");
                        ui.radio_value(&mut self.state.region, Region::Pal, "PAL");
                    });

                    ui.separator();

                    if ui.button("Restore Defaults").clicked() {
                        self.state = Default::default();
                        self.hydrate();
                    }
                });
                ui.menu_button("Windows", |ui| {
                    ui.checkbox(&mut self.state.show_screen, "Screen");
                    if ui
                        .checkbox(&mut self.state.show_nametables, "Nametables")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    if ui
                        .checkbox(&mut self.state.show_chr_tiles, "CHR Tiles")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    if ui
                        .checkbox(&mut self.state.show_cpu_mem, "CPU Memory")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    if ui
                        .checkbox(&mut self.state.show_ppu_mem, "PPU Memory")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                });
                ui.separator();

                match VolumePicker::new(&mut self.state.mute, &mut self.state.volume).ui(ui) {
                    Some(VolumeChanged::Mute) => self.set_volume(0.0),
                    Some(VolumeChanged::Volume(v)) => self.set_volume(v),
                    _ => (),
                }

                ui.separator();
                ui.label("Debug Update Freq.");
                if egui::Slider::new(&mut self.state.debug_interval, 0..=120)
                    .ui(ui)
                    .changed()
                {
                    self.update_debug_req();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |_| {});

        self.debug.swap();
        if self.state.show_screen {
            if let Some(res) = self.nes_screen.show(&ctx) {
                self.state.focus_screen = res.has_focus();
            } else {
                self.state.focus_screen = false;
            }
        }

        if self.state.show_cpu_mem {
            MemoryViewer::new("CPU Memory", self.debug.cpu_mem()).show(ctx);
        }

        if self.state.show_ppu_mem {
            MemoryViewer::new("PPU Memory", self.debug.ppu_mem()).show(ctx);
        }

        if self.state.show_chr_tiles {
            self.chr_tiles.show(
                &mut self.state.selected_palette,
                &self.debug,
                self.state.debug_interval,
                ctx,
            );
        }

        if self.state.show_nametables {
            self.nt_viewer
                .show(&self.debug, self.state.debug_interval, ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let state = ron::to_string(&self.state);
        if let Ok(state) = state {
            storage.set_string("debugger_ui_state", state);
            storage.flush();
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let input_iter = raw_input.events.iter().filter_map(|ev| {
            if let &Event::Key { key, pressed, .. } = ev {
                Some(Input { key, pressed })
            } else {
                None
            }
        });

        self.input.update(input_iter);
        let state = self.input.state();
        self.handle_input(state);

        // capture tabs to prevent focus changing during rewind while screen is focused
        if self.state.focus_screen {
            raw_input.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: egui::Key::Tab,
                        ..
                    }
                )
            });
        }
    }
}

fn spawn_machine_thread(
    emu_commands: EmulatorCommands,
    back_buffer: GfxBackBuffer,
    samples: SamplesProducer,
    sample_rate: u32,
    debug: DebugSwapState,
) {
    std::thread::spawn(move || {
        let runner =
            runner::Runner::new(emu_commands, back_buffer, Some(samples), sample_rate, debug);
        runner.run()
    });
}

fn spawn_sync_thread(input: SharedInput, emu_control: EmulatorControl, mut sync: CpalSync) {
    std::thread::spawn(move || loop {
        sync.sync_frame();
        let input = input.state();
        emu_control.player_one(input.controller);
        emu_control.sync();
        if input.rewind {
            emu_control.rewind();
        }
    });
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    RomLoaded(std::path::PathBuf),
}

pub struct AppEvents {
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
}

impl AppEvents {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }

    pub fn create_proxy(&self) -> AppEventsProxy {
        AppEventsProxy {
            tx: self.tx.clone(),
        }
    }

    pub fn send(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    pub fn poll_event(&self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
    }
}

#[derive(Debug, Clone)]
pub struct AppEventsProxy {
    tx: Sender<AppEvent>,
}

impl AppEventsProxy {
    pub fn send(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorControl {
    tx: Sender<EmulatorInput>,
}

impl EmulatorControl {
    pub fn new() -> (EmulatorControl, EmulatorCommands) {
        let (tx, rx) = channel();
        (EmulatorControl { tx }, EmulatorCommands { rx })
    }

    pub fn player_one(&self, controller: nes::Controller) {
        let _ = self
            .tx
            .send(EmulatorInput::Nes(UserInput::PlayerOne(controller)));
    }

    pub fn load_rom(&self, region: nes::Region, rom: Vec<u8>) {
        let _ = self.tx.send(EmulatorInput::LoadCartridge(region, rom));
    }

    pub fn sync(&self) {
        let _ = self.tx.send(EmulatorInput::Sync);
    }

    pub fn rewind(&self) {
        let _ = self.tx.send(EmulatorInput::Rewind);
    }

    pub fn debug_request(&self, debug: DebugRequest) {
        let _ = self.tx.send(EmulatorInput::DebugRequest(debug));
    }
}

pub struct EmulatorCommands {
    rx: Receiver<EmulatorInput>,
}

impl EmulatorCommands {
    pub fn commands(&mut self) -> impl Iterator<Item = EmulatorInput> + '_ {
        self.rx.iter()
    }
}

pub struct Input<I: Into<InputType>> {
    key: I,
    pressed: bool,
}

pub struct InputState {
    pub controller: nes::Controller,
    pub rewind: bool,
    pub power: bool,
    pub reset: bool,
    pub pause: bool,
}

#[derive(Clone)]
pub struct SharedInput {
    input_map: Arc<Mutex<InputMap>>,
}

impl SharedInput {
    pub fn new() -> Self {
        SharedInput {
            input_map: Arc::new(Mutex::new(InputMap::new())),
        }
    }

    pub fn update<I: Iterator<Item = Input<K>>, K: Into<InputType>>(&self, inputs: I) {
        let mut input_map = self.input_map.lock().unwrap();

        for input in inputs {
            if input.pressed {
                input_map.press(input.key);
            } else {
                input_map.release(input.key);
            }
        }
    }

    pub fn state(&self) -> InputState {
        let input_map = self.input_map.lock().unwrap();

        InputState {
            controller: input_map.controller(),
            rewind: input_map.rewind(),
            power: input_map.power(),
            reset: input_map.reset(),
            pause: input_map.pause(),
        }
    }
}
