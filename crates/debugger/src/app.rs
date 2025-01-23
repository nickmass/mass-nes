use eframe::{
    egui::{Event, Widget},
    CreationContext,
};
use nes::UserInput;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use ui::{
    audio::{Audio, SamplesSender},
    filters::NesNtscSetup,
    gamepad::{GamepadChannel, GamepadEvent, GilrsInput},
    input::{InputMap, InputType},
};

use std::{
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::debug_state::{DebugSwapState, DebugUiState, Palette};
use crate::egui;
use crate::gfx::{Filter, Gfx, GfxBackBuffer};
use crate::runner::{DebugRequest, EmulatorInput};
use crate::widgets::*;
use crate::{
    gfx::Repainter,
    spawner::{MachineSpawner, Spawn},
};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default = "Default::default")]
struct UiState {
    region: Region,
    volume: f32,
    mute: bool,
    show_screen: bool,
    show_nametables: bool,
    show_cpu_mem: bool,
    show_ppu_mem: bool,
    show_chr_tiles: bool,
    show_sprites: bool,
    show_all_sprites: bool,
    show_messages: bool,
    show_code: bool,
    auto_open_most_recent: bool,
    recent_files: Vec<PathBuf>,
    debug_interval: u64,
    selected_palette: u8,
    filter: Filter,
    bios: Option<Vec<u8>>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            region: Region::Ntsc,
            volume: 1.0,
            mute: false,
            show_screen: false,
            show_nametables: false,
            show_cpu_mem: false,
            show_ppu_mem: false,
            show_chr_tiles: false,
            show_sprites: false,
            show_all_sprites: false,
            show_messages: false,
            show_code: false,
            auto_open_most_recent: true,
            recent_files: Vec::new(),
            debug_interval: 10,
            selected_palette: 0,
            filter: Filter::Ntsc,
            bios: None,
        }
    }
}

pub struct DebuggerApp<A> {
    app_events: AppEvents,
    input: SharedInput,
    emu_control: EmulatorControl,
    audio: A,
    nes_screen: NesScreen,
    recents: Recents,
    last_input: InputState,
    pause: bool,
    debug: DebugUiState,
    state: UiState,
    chr_tiles: ChrTiles,
    nt_viewer: NametableViewer,
    sprite_viewer: SpriteViewer,
    messages: Messages,
    code_viewer: CodeViewer,
    breakpoints: Breakpoints,
    first_update: bool,
    help: Help,
    fds_disk_sides: usize,
    fds_current_side: Option<usize>,
}

impl<A: Audio> DebuggerApp<A> {
    pub fn new(
        cc: &CreationContext,
        message_store: MessageStore,
        audio: A,
        samples_tx: SamplesSender,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let ntsc_setup = NesNtscSetup::composite();
        let palette = ntsc_setup.generate_palette();

        let gl = cc.gl.as_ref().expect("require glow opengl context").clone();
        let app_events = AppEvents::new();
        let back_buffer = GfxBackBuffer::new(Repainter::new(cc.egui_ctx.clone()));
        let gfx = Gfx::new(gl, back_buffer.clone(), &palette)?;
        let nes_screen = NesScreen::new(gfx);
        let palette = Palette::new(palette);

        let input = SharedInput::new();
        let last_input = input.state();
        let (emu_control, emu_commands) = EmulatorControl::new(app_events.create_proxy());
        let gamepad_channel = GilrsInput::new(app_events.create_proxy()).ok();
        let debug_swap = DebugSwapState::new();
        let debug = DebugUiState::new(debug_swap.clone(), palette);
        let chr_tiles = ChrTiles::new();
        let nt_viewer = NametableViewer::new();
        let sprite_viewer = SpriteViewer::new();
        let messages = Messages::new(message_store);
        let help = Help::new(app_events.create_proxy(), emu_control.clone());

        let machine = MachineSpawner {
            emu_commands,
            back_buffer,
            samples_tx,
            sample_rate: audio.sample_rate(),
            debug: debug_swap,
        };
        machine.spawn();

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(gamepad) = gamepad_channel {
            let gamepad = crate::spawner::GamepadSpawner { gamepad };
            gamepad.spawn();
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(mut gamepad) = gamepad_channel {
            use futures::StreamExt;
            let gamepad_poll = async move {
                let mut stream = gloo::timers::future::IntervalStream::new(1);
                while let Some(_) = stream.next().await {
                    gamepad.poll();
                }
            };

            wasm_bindgen_futures::spawn_local(gamepad_poll);
        }

        let state = if let Some(storage) = cc.storage {
            storage
                .get_string("debugger_ui_state")
                .and_then(|s| ron::from_str(&s).ok())
        } else {
            None
        };

        let state: UiState = state.unwrap_or_default();

        let mut app = DebuggerApp {
            first_update: true,
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
            sprite_viewer,
            messages,
            code_viewer: CodeViewer::new(),
            breakpoints: Breakpoints::new(),
            recents: Recents::new(&[], 10),
            help,
            fds_disk_sides: 0,
            fds_current_side: None,
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
        self.nes_screen.filter(self.state.filter);

        if self.state.auto_open_most_recent {
            if let Some(recent) = self.state.recent_files.first() {
                self.load_rom(recent.clone(), self.state.bios.clone())
            }
        }

        self.update_debug_req();
    }

    fn select_rom(&self) {
        let control = self.emu_control.clone();
        let region = self.state.region;
        let last_dir = self
            .state
            .recent_files
            .first()
            .and_then(|r| r.parent())
            .map(|p| p.to_owned());
        let proxy = self.app_events.create_proxy();
        pick_file(proxy, control, region, last_dir, self.state.bios.clone());
    }

    fn select_bios(&self) {
        let proxy = self.app_events.create_proxy();
        pick_bios(proxy);
    }

    fn set_volume(&mut self, value: f32) {
        self.audio.volume(value);
    }

    fn handle_input(&mut self, input_state: InputState) {
        if !self.last_input.pause && input_state.pause {
            self.pause = !self.pause;
            self.handle_pause();
        }

        if !self.last_input.step_backward && input_state.step_backward {
            self.pause = true;
            self.handle_pause();
            self.emu_control.step_back();
            self.nes_screen.set_message(Message::StepBack);
        }

        if !self.last_input.step_forward && input_state.step_forward {
            self.pause = true;
            self.handle_pause();
            self.emu_control.step_forward();
            self.nes_screen.set_message(Message::StepForward);
        }

        self.emu_control.player_one(input_state.controller);

        if let Some(slot) = input_state.save_state {
            self.emu_control.save_state(slot);
            self.nes_screen.set_message(Message::SaveState(slot));
        }
        if let Some(slot) = input_state.restore_state {
            self.emu_control.restore_state(slot);
            self.nes_screen.set_message(Message::RestoreState(slot));
        }
        if input_state.fast_forward {
            self.emu_control.fast_forward();
            self.nes_screen.set_message(Message::FastForward);
        }
        if input_state.rewind != self.last_input.rewind {
            self.emu_control.rewind(input_state.rewind);
        }
        if input_state.rewind {
            self.nes_screen.set_message(Message::Rewind);
        }
        if input_state.power {
            self.emu_control.power();
            self.nes_screen.set_message(Message::Power);
        }
        if input_state.reset {
            self.emu_control.reset();
            self.nes_screen.set_message(Message::Reset);
        }

        self.last_input = input_state;

        if !self.nes_screen.has_message() && self.pause {
            self.nes_screen.set_message(Message::Pause);
        }
    }

    fn handle_pause(&mut self) {
        if self.pause {
            self.audio.pause();
        } else {
            self.audio.play();
        }
    }

    fn process_app_events(&mut self, ctx: &egui::Context) {
        while let Some(ev) = self.app_events.poll_event() {
            self.handle_app_event(ev, ctx);
        }
    }

    fn handle_app_event(&mut self, event: AppEvent, ctx: &egui::Context) {
        match event {
            AppEvent::BiosLoaded(bios) => {
                self.state.bios = Some(bios);
            }
            AppEvent::RomLoaded(path) => {
                self.recents.add(path);
                self.state.recent_files = self.recents.iter().map(|p| p.to_path_buf()).collect();
                self.audio.play();
            }
            AppEvent::FocusScreen => {
                self.nes_screen.focus(ctx);
            }
            AppEvent::Breakpoint => {
                self.pause = true;
                self.handle_pause();
            }
            AppEvent::Gamepad(gamepad) => match gamepad {
                GamepadEvent::Button { state, button, .. } => {
                    let Some(mut input) = self.input.input_map.try_lock().ok() else {
                        return;
                    };
                    if state.is_pressed() {
                        input.press(button);
                    } else {
                        input.release(button);
                    }
                }
                GamepadEvent::Axis { axis, value, .. } => {
                    let Some(mut input) = self.input.input_map.try_lock().ok() else {
                        return;
                    };
                    input.axis(axis, value);
                }
                _ => (),
            },
            AppEvent::CartridgeInfo(cartridge_kind) => match cartridge_kind {
                CartridgeKind::Cartridge => {
                    self.fds_disk_sides = 0;
                    self.fds_current_side = None;
                }
                CartridgeKind::Fds {
                    current_side,
                    total_sides,
                } => {
                    self.fds_disk_sides = total_sides;
                    self.fds_current_side = current_side;
                }
            },
        }
    }

    fn load_rom(&self, rom_file: PathBuf, bios: Option<Vec<u8>>) {
        if let Some(bytes) = std::fs::read(&rom_file).ok() {
            let file_name = rom_file
                .file_name()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
                .unwrap_or(String::new());
            self.emu_control
                .load_rom(self.state.region.into(), bytes, file_name, bios);
            self.app_events.send(AppEvent::RomLoaded(rom_file));
        }
    }

    fn update_debug_req(&self) {
        let mut debug = DebugRequest {
            interval: self.state.debug_interval,
            cpu_mem: self.state.show_cpu_mem | self.state.show_code,
            ppu_mem: self.state.show_ppu_mem
                | self.state.show_chr_tiles
                | self.state.show_nametables
                | self.state.show_sprites,
            pal_ram: self.state.show_chr_tiles
                | self.state.show_nametables
                | self.state.show_sprites,
            sprite_ram: self.state.show_sprites,
            state: self.state.show_code | self.state.show_nametables | self.state.show_sprites,
            breakpoints: self.breakpoints.clone(),
        };

        if !debug.cpu_mem && !debug.ppu_mem && !debug.pal_ram && !debug.sprite_ram && !debug.state {
            debug.interval = 0;
        }

        self.emu_control.debug_request(debug);
    }
}

impl<A: Audio> eframe::App for DebuggerApp<A> {
    #[instrument(skip_all)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.debug.breakpoint() {
            self.app_events.send(AppEvent::Breakpoint);
        }

        self.process_app_events(ctx);

        egui::TopBottomPanel::top("Menu Area").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.select_rom();
                        ui.close_menu();
                    }
                    if let Some(file) = self.recents.ui(ui) {
                        self.load_rom(file.to_path_buf(), self.state.bios.clone());
                    }

                    ui.menu_button("Region", |ui| {
                        ui.radio_value(&mut self.state.region, Region::Ntsc, "NTSC");
                        ui.radio_value(&mut self.state.region, Region::Pal, "PAL");
                    });

                    if ui.button("Load Bios").clicked() {
                        self.select_bios();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Restore Defaults").clicked() {
                        self.state = Default::default();
                        self.hydrate();
                    }
                });
                ui.menu_button("Game", |ui| {
                    if ui.button("Power Cycle").clicked() {
                        self.emu_control.power();
                    }
                    if ui.button("Reset").clicked() {
                        self.emu_control.reset();
                    }

                    if self.fds_disk_sides > 0 {
                        ui.menu_button("Select Disk", |ui| {
                            let mut changed = ui
                                .radio_value(&mut self.fds_current_side, None, "Eject")
                                .changed();
                            ui.separator();
                            for disk_side in 0..self.fds_disk_sides {
                                let side = if disk_side % 2 == 0 { "A" } else { "B" };
                                let disk = (disk_side / 2) + 1;

                                changed |= ui
                                    .radio_value(
                                        &mut self.fds_current_side,
                                        Some(disk_side),
                                        format!("Disk {disk} Side {side}"),
                                    )
                                    .changed();
                            }

                            if changed {
                                self.emu_control.set_disk_side(self.fds_current_side);
                            }
                        });
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
                    if ui.checkbox(&mut self.state.show_code, "Code").changed() {
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
                    if ui
                        .checkbox(&mut self.state.show_sprites, "Sprites")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    ui.checkbox(&mut self.state.show_messages, "Messages");
                });
                ui.menu_button("Filter", |ui| {
                    if ui
                        .radio_value(&mut self.state.filter, Filter::Paletted, "None")
                        .changed()
                    {
                        self.nes_screen.filter(self.state.filter);
                    }
                    if ui
                        .radio_value(&mut self.state.filter, Filter::Ntsc, "NTSC")
                        .changed()
                    {
                        self.nes_screen.filter(self.state.filter);
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

        self.debug.swap();

        let bg = if self.state.show_screen {
            egui::Frame::central_panel(&*ctx.style())
        } else {
            egui::Frame::none()
                .inner_margin(0.0)
                .outer_margin(0.0)
                .fill(egui::Color32::BLACK)
        };

        egui::CentralPanel::default().frame(bg).show(ctx, |ui| {
            if !self.state.show_screen {
                ui.centered_and_justified(|ui| {
                    self.nes_screen.fill(ctx, ui);
                });
            }
        });

        if self.state.show_screen {
            self.nes_screen.show(&ctx);
        }

        self.help.show(&ctx);

        if self.state.show_code {
            let mut paused = self.pause;
            if self
                .code_viewer
                .show(&mut paused, &self.debug, &mut self.breakpoints, ctx)
            {
                self.update_debug_req();
            }

            if paused != self.pause {
                self.pause = paused;
                self.handle_pause();
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

        if self.state.show_sprites {
            self.sprite_viewer.show(
                &mut self.state.show_all_sprites,
                &self.debug,
                self.state.debug_interval,
                ctx,
            );
        }

        if self.state.show_messages {
            self.messages.show(ctx);
        }

        if self.first_update {
            self.first_update = false;
            self.app_events.send(AppEvent::FocusScreen);
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

        if let Some(state) = self.input.update(input_iter) {
            self.handle_input(state);
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    RomLoaded(std::path::PathBuf),
    FocusScreen,
    Breakpoint,
    Gamepad(GamepadEvent),
    BiosLoaded(Vec<u8>),
    CartridgeInfo(CartridgeKind),
}

impl From<GamepadEvent> for AppEvent {
    fn from(value: GamepadEvent) -> Self {
        Self::Gamepad(value)
    }
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

impl GamepadChannel for AppEventsProxy {
    type Event = AppEvent;

    type Err = ();

    fn send_event(&self, event: Self::Event) -> Result<(), Self::Err> {
        Ok(self.send(event))
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorControl {
    tx: Sender<EmulatorInput>,
}

impl EmulatorControl {
    pub fn new(proxy: AppEventsProxy) -> (EmulatorControl, EmulatorCommands) {
        let (tx, rx) = channel();
        (EmulatorControl { tx }, EmulatorCommands { rx, proxy })
    }

    pub fn player_one(&self, controller: nes::Controller) {
        let _ = self
            .tx
            .send(EmulatorInput::Nes(UserInput::PlayerOne(controller)));
    }

    pub fn load_rom(
        &self,
        region: nes::Region,
        rom: Vec<u8>,
        file_name: String,
        bios: Option<Vec<u8>>,
    ) {
        let _ = self
            .tx
            .send(EmulatorInput::LoadCartridge(region, rom, file_name, bios));
    }

    pub fn step_back(&self) {
        let _ = self.tx.send(EmulatorInput::StepBack);
    }

    pub fn step_forward(&self) {
        let _ = self.tx.send(EmulatorInput::StepForward);
    }

    pub fn rewind(&self, toggle: bool) {
        let _ = self.tx.send(EmulatorInput::Rewind(toggle));
    }

    pub fn power(&self) {
        let _ = self.tx.send(EmulatorInput::Nes(UserInput::Power));
    }

    pub fn reset(&self) {
        let _ = self.tx.send(EmulatorInput::Nes(UserInput::Reset));
    }

    pub fn debug_request(&self, debug: DebugRequest) {
        let _ = self.tx.send(EmulatorInput::DebugRequest(debug));
    }

    pub fn fast_forward(&self) {
        let _ = self.tx.send(EmulatorInput::FastForward);
    }

    pub fn restore_state(&self, slot: u8) {
        let _ = self.tx.send(EmulatorInput::RestoreState(slot as u32));
    }

    pub fn save_state(&self, slot: u8) {
        let _ = self.tx.send(EmulatorInput::SaveState(slot as u32));
    }

    pub fn set_disk_side(&self, side: Option<usize>) {
        let _ = self.tx.send(EmulatorInput::SetFdsDisk(side));
    }
}

pub struct EmulatorCommands {
    rx: Receiver<EmulatorInput>,
    proxy: AppEventsProxy,
}

impl EmulatorCommands {
    pub fn try_command(&mut self) -> Option<EmulatorInput> {
        self.rx.try_recv().ok()
    }

    pub fn send_cartridge_info(&self, cartridge: CartridgeKind) {
        self.proxy.send(AppEvent::CartridgeInfo(cartridge));
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CartridgeKind {
    Cartridge,
    Fds {
        current_side: Option<usize>,
        total_sides: usize,
    },
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
    pub step_forward: bool,
    pub step_backward: bool,
    pub save_state: Option<u8>,
    pub restore_state: Option<u8>,
    fast_forward: bool,
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

    pub fn update<I: Iterator<Item = Input<K>>, K: Into<InputType>>(
        &self,
        inputs: I,
    ) -> Option<InputState> {
        let mut input_map = self.input_map.try_lock().ok()?;

        for input in inputs {
            if input.pressed {
                input_map.press(input.key);
            } else {
                input_map.release(input.key);
            }
        }

        let state = InputState {
            controller: input_map.controller(),
            rewind: input_map.rewind(),
            power: input_map.power(),
            reset: input_map.reset(),
            pause: input_map.pause(),
            step_forward: input_map.step_forward(),
            step_backward: input_map.step_backward(),
            save_state: input_map.save_state(),
            restore_state: input_map.restore_state(),
            fast_forward: input_map.fast_forward(),
        };

        Some(state)
    }

    pub fn state(&self) -> InputState {
        let input_map = self.input_map.lock().unwrap();

        InputState {
            controller: input_map.controller(),
            rewind: input_map.rewind(),
            power: input_map.power(),
            reset: input_map.reset(),
            pause: input_map.pause(),
            step_forward: input_map.step_forward(),
            step_backward: input_map.step_backward(),
            save_state: input_map.save_state(),
            restore_state: input_map.restore_state(),
            fast_forward: input_map.fast_forward(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_file(
    proxy: AppEventsProxy,
    control: EmulatorControl,
    region: Region,
    last_dir: Option<PathBuf>,
    bios: Option<Vec<u8>>,
) {
    std::thread::spawn(move || {
        let picker = rfd::FileDialog::new()
            .add_filter("All Supported Files", &["nes", "NES", "fds", "FDS"])
            .add_filter("NES Cartridges", &["nes", "NES"])
            .add_filter("Famicom Disk System", &["fds", "FDS"]);

        let rom_file = if let Some(last_dir) = last_dir {
            picker.set_directory(last_dir).pick_file()
        } else {
            picker.pick_file()
        };

        if let Some((path, bytes)) = rom_file.and_then(|p| {
            let bytes = std::fs::read(&p).ok();
            Some(p).zip(bytes)
        }) {
            let file_name = path
                .file_name()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
                .unwrap_or(String::new());
            control.load_rom(region.into(), bytes, file_name, bios);
            proxy.send(AppEvent::RomLoaded(path));
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn pick_file(
    proxy: AppEventsProxy,
    control: EmulatorControl,
    region: Region,
    last_dir: Option<PathBuf>,
    bios: Option<Vec<u8>>,
) {
    let picker = rfd::AsyncFileDialog::new()
        .add_filter("All Supported Files", &["nes", "NES", "fds", "FDS"])
        .add_filter("NES Cartridges", &["nes", "NES"])
        .add_filter("Famicom Disk System", &["fds", "FDS"]);

    let pick = async move {
        let rom_file = if let Some(last_dir) = last_dir {
            picker.set_directory(last_dir).pick_file().await
        } else {
            picker.pick_file().await
        };

        let Some(rom_file) = rom_file else {
            return;
        };

        let bytes = rom_file.read().await;
        control.load_rom(region.into(), bytes, rom_file.file_name(), bios);
        proxy.send(AppEvent::RomLoaded(std::path::PathBuf::new()));
    };

    wasm_bindgen_futures::spawn_local(pick);
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_bios(proxy: AppEventsProxy) {
    std::thread::spawn(move || {
        let rom_file = rfd::FileDialog::new()
            .add_filter("All Supported Files", &["rom", "ROM"])
            .pick_file();

        if let Some(bytes) = rom_file.and_then(|p| std::fs::read(&p).ok()) {
            proxy.send(AppEvent::BiosLoaded(bytes));
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn pick_bios(proxy: AppEventsProxy) {
    let picker = rfd::AsyncFileDialog::new().add_filter("All Supported Files", &["rom", "ROM"]);

    let pick = async move {
        let rom_file = picker.pick_file().await;

        let Some(rom_file) = rom_file else {
            return;
        };

        let bytes = rom_file.read().await;
        proxy.send(AppEvent::BiosLoaded(bytes));
    };

    wasm_bindgen_futures::spawn_local(pick);
}
