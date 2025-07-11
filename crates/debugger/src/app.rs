use eframe::CreationContext;
use nes::{ChannelPlayback, SaveWram, UserInput};
use serde::{Deserialize, Serialize};
use ui::{
    audio::{Audio, SamplesSender},
    gamepad::{GamepadChannel, GamepadEvent, GilrsInput},
    input::{InputMap, InputType},
    wram::{CartridgeId, WramStorage},
};

use std::{
    fs::File,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
};

use crate::egui::{self, Event};
use crate::gfx::{Filter, Gfx, GfxBackBuffer};
use crate::runner::{DebugRequest, EmulatorInput, StepKind};
use crate::widgets::*;
use crate::{
    debug_state::{DebugSwapState, DebugUiState},
    platform,
};
use crate::{
    gfx::Repainter,
    spawner::{MachineSpawner, Spawn},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    Ntsc,
    Pal,
}

impl Default for Region {
    fn default() -> Self {
        Region::Ntsc
    }
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
    show_events: bool,
    show_filter_config: bool,
    show_input_viewer: bool,
    show_audio_channels: bool,
    show_variables: bool,
    auto_open_most_recent: bool,
    interests: Interests,
    recent_files: Vec<PathBuf>,
    debug_interval: u64,
    selected_palette: u8,
    filter: Filter,
    ntsc_config: NtscConfig,
    bios: Option<Vec<u8>>,
    variable_viewer: VariableViewerState,
    movie_settings: MovieSettingsState,
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
            show_events: false,
            show_filter_config: false,
            show_input_viewer: false,
            show_audio_channels: false,
            show_variables: false,
            auto_open_most_recent: true,
            interests: Interests::new(),
            recent_files: Vec::new(),
            debug_interval: 1,
            selected_palette: 0,
            filter: Filter::Crt,
            ntsc_config: NtscConfig::default(),
            bios: None,
            variable_viewer: VariableViewerState::default(),
            movie_settings: MovieSettingsState::default(),
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
    event_viewer: EventViewer,
    breakpoints: Breakpoints,
    first_update: bool,
    help: Help,
    fds_disk_sides: usize,
    fds_current_side: Option<usize>,
    wram: Option<ui::wram::WramStorage>,
    svg_renderer: svg::SvgRenderer,
    controller_svg: svg::SvgGlView,
    channel_viewer: ChannelViewer,
    variable_viewer: VariableViewer,
    movie_settings: MovieSettings,
    recording_wav: bool,
}

impl<A: Audio> DebuggerApp<A> {
    pub fn new(
        cc: &CreationContext,
        message_store: MessageStore,
        audio: A,
        samples_tx: SamplesSender,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Force dark mode as it is currently the only version I ever test
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);

        let state = if let Some(storage) = cc.storage {
            storage
                .get_string("debugger_ui_state")
                .and_then(|s| ron::from_str(&s).ok())
        } else {
            None
        };

        let state: UiState = state.unwrap_or_default();

        let gl = cc.gl.as_ref().expect("require glow opengl context").clone();
        let app_events = AppEvents::new();
        let back_buffer = GfxBackBuffer::new(Repainter::new(cc.egui_ctx.clone()));
        let gfx = Gfx::new(gl.clone(), back_buffer.clone(), state.ntsc_config.clone())?;
        let nes_screen = NesScreen::new(gfx);
        let wram = platform::wram_storage();
        let input = SharedInput::new();
        let last_input = input.state();
        let (emu_control, emu_commands) =
            EmulatorControl::new(app_events.create_proxy(), wram.clone());
        let gamepad_channel = GilrsInput::new(app_events.create_proxy()).ok();
        let debug_swap = DebugSwapState::new();
        let debug = DebugUiState::new(debug_swap.clone(), state.ntsc_config.clone());
        let chr_tiles = ChrTiles::new();
        let nt_viewer = NametableViewer::new();
        let sprite_viewer = SpriteViewer::new();
        let messages = Messages::new(message_store);
        let help = Help::new(app_events.create_proxy(), emu_control.clone());
        let svg_renderer = svg::SvgRenderer::new(gl).unwrap();

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
            event_viewer: EventViewer::new(),
            breakpoints: Breakpoints::new(),
            recents: Recents::new(&[], 10),
            help,
            fds_disk_sides: 0,
            fds_current_side: None,
            wram,
            controller_svg: svg::nes_controller().with_scale(1.0),
            svg_renderer,
            channel_viewer: ChannelViewer::new(),
            variable_viewer: VariableViewer::new(),
            movie_settings: MovieSettings::new(),
            recording_wav: false,
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
        pick_file(
            proxy,
            control,
            region,
            last_dir,
            self.state.bios.clone(),
            self.state.movie_settings.restore_wram,
        );
    }

    fn select_bios(&self) {
        let proxy = self.app_events.create_proxy();
        pick_bios(proxy);
    }

    fn select_movie(&self) {
        let proxy = self.app_events.create_proxy();
        pick_movie(proxy);
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
            self.emu_control.step_forward(StepKind::Frame);
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
        if input_state.fast_forward != self.last_input.fast_forward {
            self.emu_control.fast_forward(input_state.fast_forward);
        }
        if input_state.fast_forward {
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
            self.debug.clear_breakpoint();
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
                self.emu_control
                    .channel_playback(self.channel_viewer.playback());
                self.recents.add(path);
                self.state.recent_files = self.recents.iter().map(|p| p.to_path_buf()).collect();
                self.pause = false;
                self.handle_pause();
                self.nes_screen.focus(ctx);
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
            AppEvent::SaveWram(cart, wram) => {
                if let Some(store) = self.wram.as_ref() {
                    if self.state.movie_settings.restore_wram {
                        store.save_wram(cart, wram);
                    }
                }
            }
            AppEvent::MovieLoaded(file_name, bytes) => {
                let file = std::io::Cursor::new(bytes);
                let offset = self.state.movie_settings.frame_offset;
                let movie = if file_name.ends_with(".fm2") {
                    Some(ui::movie::MovieFile::fm2(file, offset))
                } else if file_name.ends_with(".bk2") {
                    Some(ui::movie::MovieFile::bk2(file, offset))
                } else if file_name.ends_with(".zip") {
                    if let Some(bytes) = read_first_match_in_zip(".fm2", file.clone()) {
                        let file = std::io::Cursor::new(bytes);
                        Some(ui::movie::MovieFile::fm2(file, offset))
                    } else if let Some(bytes) = read_first_match_in_zip(".bk2", file) {
                        let file = std::io::Cursor::new(bytes);
                        Some(ui::movie::MovieFile::bk2(file, offset))
                    } else {
                        None
                    }
                } else {
                    None
                };

                match movie {
                    Some(Ok(movie)) => self.emu_control.play_movie(movie),
                    Some(Err(e)) => tracing::error!("Unable to parse movie file: {:?}", e),
                    None => tracing::warn!("No movie found in file"),
                }
            }
            AppEvent::PickWav(path_buf) => {
                if let Ok(file) = File::create(path_buf) {
                    self.recording_wav = true;
                    self.emu_control.record_wav(file);
                } else {
                    tracing::error!("unable to create wav file");
                }
            }
        }
    }

    fn load_rom(&self, rom_file: PathBuf, bios: Option<Vec<u8>>) {
        if let Some(bytes) = std::fs::read(&rom_file).ok() {
            let file_name = rom_file
                .file_name()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
                .unwrap_or(String::new());
            self.emu_control.load_rom(
                self.state.region.into(),
                bytes,
                file_name,
                bios,
                self.state.movie_settings.restore_wram,
            );
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
            events: self.state.show_events,
            interests: self.state.interests.events().collect(),
            interest_breakpoints: self.state.interests.breakpoint_mask(),
            frame: self.state.show_events,
            channels: self.state.show_audio_channels,
        };

        if !debug.cpu_mem
            && !debug.ppu_mem
            && !debug.pal_ram
            && !debug.sprite_ram
            && !debug.state
            && !debug.events
            && !debug.frame
            && !debug.channels
            && !self.state.show_variables
        {
            debug.interval = 0;
        }

        self.emu_control.debug_request(debug);
    }
}

impl<A: Audio> eframe::App for DebuggerApp<A> {
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

                    if ui.button("Load Movie").clicked() {
                        self.select_movie();
                        ui.close_menu();
                    }

                    if ui.button("Movie Settings").clicked() {
                        self.state.movie_settings.show_settings = true;
                        ui.close_menu();
                    }

                    if cfg!(not(target_arch = "wasm32")) {
                        if self.recording_wav {
                            if ui.button("End Recording").clicked() {
                                self.emu_control.stop_record_wav();
                                self.recording_wav = false;
                                ui.close_menu();
                            }
                        } else {
                            if ui.button("Record WAV").clicked() {
                                pick_wav(self.app_events.create_proxy());
                                ui.close_menu();
                            }
                        }
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
                    if ui.checkbox(&mut self.state.show_events, "Events").changed() {
                        self.update_debug_req();
                    }
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
                    if ui
                        .checkbox(&mut self.state.show_audio_channels, "Audio Channels")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    if ui
                        .checkbox(&mut self.state.show_variables, "Variables")
                        .changed()
                    {
                        self.update_debug_req();
                    }
                    ui.checkbox(&mut self.state.show_input_viewer, "Input Viewer");
                    ui.checkbox(&mut self.state.show_messages, "Messages");
                });
                ui.menu_button("Filter", |ui| {
                    ui.toggle_value(&mut self.state.show_filter_config, "Configure");
                    ui.separator();
                    let old_filter = self.state.filter;

                    let filters = [
                        (Filter::Paletted, "None"),
                        (Filter::Ntsc, "NTSC"),
                        (Filter::Crt, "CRT"),
                    ];

                    for (filter, label) in filters {
                        if ui
                            .radio_value(&mut self.state.filter, filter, label)
                            .changed()
                        {
                            if !self.nes_screen.filter(self.state.filter) {
                                self.state.filter = old_filter;
                            }
                        }
                    }
                });
                ui.separator();

                match VolumePicker::new(&mut self.state.mute, &mut self.state.volume).ui(ui) {
                    Some(VolumeChanged::Mute) => self.set_volume(0.0),
                    Some(VolumeChanged::Volume(v)) => self.set_volume(v),
                    _ => (),
                }
            });
        });

        self.debug.swap();

        let bg = if self.state.show_screen {
            egui::Frame::central_panel(&*ctx.style())
        } else {
            egui::Frame::new()
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

        self.movie_settings
            .show(&mut self.state.movie_settings, ctx);
        self.help.show(&ctx);

        if self.state.show_code {
            let mut paused = self.pause;
            if let Some(action) =
                self.code_viewer
                    .show(&mut paused, &self.debug, &mut self.breakpoints, ctx)
            {
                match action {
                    CodeViewerAction::UpdateBreakpoint => self.update_debug_req(),
                    CodeViewerAction::Step(step) => self.emu_control.step_forward(step),
                }
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

        if self.state.show_audio_channels {
            if let Some(playback) =
                self.channel_viewer
                    .show(ctx, &self.debug, self.state.debug_interval)
            {
                self.emu_control.channel_playback(playback);
            }
        }

        if self.state.show_variables {
            self.variable_viewer
                .show(ctx, &mut self.state.variable_viewer, &self.debug);
        }

        if self.state.show_events {
            if self.event_viewer.show(
                &self.state.region,
                &self.debug,
                self.state.debug_interval,
                &mut self.state.interests,
                ctx,
            ) {
                self.update_debug_req();
            }
        }

        if self.state.show_messages {
            self.messages.show(ctx);
        }

        if self.state.show_filter_config {
            self.nes_screen
                .configure_filter(ctx, &mut self.state.ntsc_config, &mut self.debug);
        }

        if self.state.show_input_viewer {
            egui::Window::new("Input Viewer").show(ctx, |ui| {
                let mut buttons = svg::NesButtons::empty();
                let controller = self.last_input.controller;

                if controller.up {
                    buttons |= svg::NesButtons::UP;
                }
                if controller.down {
                    buttons |= svg::NesButtons::DOWN;
                }
                if controller.left {
                    buttons |= svg::NesButtons::LEFT;
                }
                if controller.right {
                    buttons |= svg::NesButtons::RIGHT;
                }
                if controller.a {
                    buttons |= svg::NesButtons::A;
                }
                if controller.b {
                    buttons |= svg::NesButtons::B;
                }
                if controller.select {
                    buttons |= svg::NesButtons::SELECT;
                }
                if controller.start {
                    buttons |= svg::NesButtons::START;
                }

                self.controller_svg
                    .view(&self.svg_renderer, buttons.bits(), ui)
            });
        }

        if self.first_update {
            self.first_update = false;
            self.app_events.send(AppEvent::FocusScreen);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.emu_control.save_wram();
        let state = ron::to_string(&self.state);
        if let Ok(state) = state {
            storage.set_string("debugger_ui_state", state);
            storage.flush();
        }
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let input_iter = raw_input.events.iter().filter_map(|ev| {
            if let &Event::Key { key, pressed, .. } = ev {
                Some(Input { key, pressed })
            } else {
                None
            }
        });

        if ctx.memory(|m| m.has_focus(self.nes_screen.id())) {
            if let Some(state) = self.input.update(input_iter) {
                self.handle_input(state);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    RomLoaded(PathBuf),
    FocusScreen,
    Breakpoint,
    Gamepad(GamepadEvent),
    BiosLoaded(Vec<u8>),
    CartridgeInfo(CartridgeKind),
    SaveWram(CartridgeId, SaveWram),
    MovieLoaded(String, Vec<u8>),
    PickWav(PathBuf),
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
    wram: Option<WramStorage>,
    tx: Sender<EmulatorInput>,
}

impl EmulatorControl {
    pub fn new(
        proxy: AppEventsProxy,
        wram: Option<WramStorage>,
    ) -> (EmulatorControl, EmulatorCommands) {
        let (tx, rx) = channel();
        (EmulatorControl { tx, wram }, EmulatorCommands { rx, proxy })
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
        restore_wram: bool,
    ) {
        self.save_wram();
        let cart_id = ui::wram::CartridgeId::new(&rom);
        let wram = if let Some(wram) = self.wram.as_ref().filter(|_| restore_wram) {
            wram.load_wram(cart_id)
        } else {
            None
        };
        let _ = self.tx.send(EmulatorInput::LoadCartridge(
            cart_id, region, rom, file_name, wram, bios,
        ));
    }

    pub fn step_back(&self) {
        let _ = self.tx.send(EmulatorInput::StepBack);
    }

    pub fn step_forward(&self, kind: StepKind) {
        let _ = self.tx.send(EmulatorInput::StepForward(kind));
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

    pub fn fast_forward(&self, toggle: bool) {
        let _ = self.tx.send(EmulatorInput::FastForward(toggle));
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

    fn save_wram(&self) {
        let _ = self.tx.send(EmulatorInput::SaveWram);
    }

    fn channel_playback(&self, playback: ChannelPlayback) {
        let _ = self.tx.send(EmulatorInput::ChannelPlayback(playback));
    }

    fn play_movie(&self, movie: ui::movie::MovieFile) {
        let _ = self.tx.send(EmulatorInput::PlayMovie(movie));
    }

    fn record_wav(&self, file: File) {
        let _ = self.tx.send(EmulatorInput::RecordWav(file));
    }

    fn stop_record_wav(&self) {
        let _ = self.tx.send(EmulatorInput::StopRecordWav);
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

    pub fn send_wram(&self, cart_id: CartridgeId, wram: SaveWram) {
        self.proxy.send(AppEvent::SaveWram(cart_id, wram))
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
    restore_wram: bool,
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
            control.load_rom(region.into(), bytes, file_name, bios, restore_wram);
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
    restore_wram: bool,
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
        control.load_rom(
            region.into(),
            bytes,
            rom_file.file_name(),
            bios,
            restore_wram,
        );
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

#[cfg(not(target_arch = "wasm32"))]
fn pick_movie(proxy: AppEventsProxy) {
    std::thread::spawn(move || {
        let Some(movie_file) = rfd::FileDialog::new()
            .add_filter("All Supported Files", &["fm2", "bk2", "zip"])
            .pick_file()
        else {
            return;
        };

        let Some(name) = movie_file.file_name().map(|n| n.to_string_lossy().into()) else {
            return;
        };

        if let Some(bytes) = std::fs::read(movie_file).ok() {
            proxy.send(AppEvent::MovieLoaded(name, bytes));
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn pick_movie(proxy: AppEventsProxy) {
    let picker =
        rfd::AsyncFileDialog::new().add_filter("All Supported Files", &["fm2", "bk2", "zip"]);

    let pick = async move {
        let movie_file = picker.pick_file().await;

        let Some(movie_file) = movie_file else {
            return;
        };

        let name = movie_file.file_name();
        let bytes = movie_file.read().await;

        proxy.send(AppEvent::MovieLoaded(name, bytes));
    };

    wasm_bindgen_futures::spawn_local(pick);
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_wav(proxy: AppEventsProxy) {
    std::thread::spawn(move || {
        let Some(wav_file) = rfd::FileDialog::new()
            .set_file_name("recording.wav")
            .add_filter("All Supported Files", &["wav"])
            .save_file()
        else {
            return;
        };

        proxy.send(AppEvent::PickWav(wav_file));
    });
}

#[cfg(target_arch = "wasm32")]
fn pick_wav(proxy: AppEventsProxy) {
    // unsupported
}

fn read_first_match_in_zip<R: std::io::Read + std::io::Seek>(
    extension: &str,
    read: R,
) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(read).ok()?;
    let mut file_match = None;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).ok()?;
        if file.name().ends_with(extension) {
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf).ok()?;
            file_match = Some(buf);
            break;
        }
    }

    file_match
}
