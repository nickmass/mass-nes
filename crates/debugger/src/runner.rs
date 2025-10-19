use std::{collections::VecDeque, time::Duration};

use blip_buf::BlipBuf;
use nes::{
    Cartridge, DebugEvent, FdsInput, InputSource, Machine, MapperInput, Region, RunResult,
    SaveWram, SimpleInput, UserInput,
    run_until::{self, RunUntil},
};
use ui::{audio::SamplesSender, movie::MovieFile, wram::CartridgeId};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StepKind {
    Dot,
    Cycle,
    Instruction,
    Scanline,
    Frame,
}

impl StepKind {
    pub fn all() -> &'static [StepKind] {
        &[
            StepKind::Dot,
            StepKind::Cycle,
            StepKind::Instruction,
            StepKind::Scanline,
            StepKind::Frame,
        ]
    }
}

impl std::fmt::Display for StepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepKind::Dot => write!(f, "Dot"),
            StepKind::Cycle => write!(f, "Cycle"),
            StepKind::Instruction => write!(f, "Instruction"),
            StepKind::Scanline => write!(f, "Scanline"),
            StepKind::Frame => write!(f, "Frame"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Playback {
    Normal,
    Rewind,
    StepForward,
    StepBackward,
    FastForward,
}

impl Playback {
    fn save_state(&self) -> bool {
        match self {
            Playback::Rewind | Playback::StepBackward => false,
            _ => true,
        }
    }

    fn update_audio(&self) -> bool {
        match self {
            Playback::StepForward | Playback::StepBackward | Playback::FastForward => false,
            _ => true,
        }
    }

    fn skip_step(&self) -> bool {
        match self {
            Playback::StepForward | Playback::StepBackward => true,
            _ => false,
        }
    }

    fn skip_sleep(&self) -> bool {
        match self {
            Playback::FastForward | Playback::StepForward | Playback::StepBackward => true,
            _ => false,
        }
    }

    fn frame_freq(&self) -> usize {
        match self {
            Playback::FastForward => 2,
            _ => 1,
        }
    }

    fn reverse(&self) -> bool {
        match self {
            Playback::Rewind | Playback::StepBackward => true,
            _ => false,
        }
    }

    fn update_debug(&self) -> bool {
        match self {
            Playback::StepForward | Playback::StepBackward => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum EmulatorInput {
    Nes(UserInput),
    Rewind(bool),
    SaveState(u32),
    RestoreState(u32),
    LoadCartridge(
        CartridgeId,
        Region,
        Vec<u8>,
        String,
        Option<SaveWram>,
        Option<Vec<u8>>,
        bool,
    ),
    DebugRequest(DebugRequest),
    StepBack,
    StepForward(StepKind),
    FastForward(bool),
    SetFdsDisk(Option<usize>),
    SaveWram,
    PlayMovie(MovieFile),
    ChannelPlayback(nes::ChannelPlayback),
    RecordWav(std::fs::File),
    StopRecordWav,
}

#[derive(Debug)]
pub struct DebugRequest {
    pub interval: u64,
    pub cpu_mem: bool,
    pub ppu_mem: bool,
    pub pal_ram: bool,
    pub sprite_ram: bool,
    pub state: bool,
    pub breakpoints: Breakpoints,
    pub events: bool,
    pub interests: Vec<DebugEvent>,
    pub interest_breakpoints: u16,
    pub frame: bool,
    pub channels: bool,
    pub variables: bool,
    pub inputs: bool,
}

use crate::{
    app::{CartridgeKind, EmulatorCommands},
    debug_state::DebugSwapState,
    gfx::GfxBackBuffer,
    widgets::Breakpoints,
};

pub struct Runner {
    machine: Option<Machine>,
    cart_id: Option<CartridgeId>,
    back_buffer: GfxBackBuffer,
    commands: EmulatorCommands,
    movie_input: Option<MovieFile>,
    samples_tx: SamplesSender,
    sample_rate: u32,
    blip: BlipBuf,
    blip_delta: i32,
    save_states: Vec<Option<(usize, nes::SaveData)>>,
    save_store: SaveStore,
    frame: usize,
    total_frames: u64,
    debug: DebugSwapState,
    debug_request: DebugRequest,
    input_source: SimpleInput,
}

impl Runner {
    pub fn new(
        commands: EmulatorCommands,
        back_buffer: GfxBackBuffer,
        samples_tx: SamplesSender,
        sample_rate: u32,
        debug: DebugSwapState,
    ) -> Self {
        let blip = BlipBuf::new(sample_rate);

        let save_store = SaveStore::builder()
            .add(1, 600)
            .add(2, 3600)
            .add(2, 600)
            .add(2, 600)
            .add(2, 10000)
            .add(2, 10000)
            .build();

        Self {
            machine: None,
            cart_id: None,
            back_buffer,
            commands,
            samples_tx,
            sample_rate,
            blip,
            blip_delta: 0,
            save_states: vec![None; 10],
            save_store,
            frame: 0,
            total_frames: 0,
            debug,
            debug_request: DebugRequest {
                interval: 0,
                cpu_mem: false,
                ppu_mem: false,
                pal_ram: false,
                sprite_ram: false,
                state: false,
                events: false,
                breakpoints: Breakpoints::new(),
                interests: Vec::new(),
                interest_breakpoints: 0,
                frame: false,
                channels: false,
                variables: false,
                inputs: false,
            },
            movie_input: None,
            input_source: SimpleInput::new(),
        }
    }

    pub fn run(mut self) {
        let mut rewinding = false;
        let mut fast_forwarding = false;
        let mut max_step = nes::Region::Ntsc.cpu_clock().ceil() as u32;
        let mut samples_per_frame =
            (self.sample_rate as f64 / nes::Region::Ntsc.refresh_rate()).ceil() as usize;
        loop {
            let mut playback = Playback::Normal;
            while let Some(input) = self.commands.try_command() {
                match input {
                    EmulatorInput::Nes(input) => {
                        if self.movie_input.is_none() {
                            self.input_source.handle_input(input)
                        }
                    }
                    EmulatorInput::SaveState(slot) => {
                        if let Some(machine) = &self.machine {
                            let data = machine.save_state();

                            self.save_states[slot as usize] = Some((self.frame, data));
                        }
                    }
                    EmulatorInput::RestoreState(slot) => {
                        if let Some(machine) = &mut self.machine {
                            if let Some((_frame, data)) = self.save_states[slot as usize].as_ref() {
                                machine.restore_state(data);
                                self.frame = machine.frame() as usize;
                            }
                        }
                    }
                    EmulatorInput::Rewind(toggle) => rewinding = toggle,
                    EmulatorInput::StepBack => {
                        if let Some(machine) = &mut self.machine {
                            if let Some((_frame, data)) = self.save_store.pop() {
                                machine.restore_state(&data);
                                self.frame = machine.frame() as usize;
                                playback = Playback::StepBackward;
                                self.step(playback, run_until::Frames(1));
                            }
                        }
                    }
                    EmulatorInput::StepForward(kind) => {
                        playback = Playback::StepForward;
                        match kind {
                            StepKind::Dot => {
                                self.step(playback, run_until::Dots(1));
                            }
                            StepKind::Cycle => {
                                self.step(playback, run_until::Cycles(1));
                            }
                            StepKind::Instruction => {
                                self.step(playback, run_until::Instructions(1));
                            }
                            StepKind::Scanline => {
                                self.step(playback, run_until::Scanlines(1));
                            }
                            StepKind::Frame => {
                                self.step(playback, run_until::Frames(1));
                            }
                        }
                    }
                    EmulatorInput::LoadCartridge(
                        cart_id,
                        region,
                        rom,
                        file_name,
                        wram,
                        bios,
                        game_genie,
                    ) => {
                        let mut rom = std::io::Cursor::new(rom);
                        let mut bios = bios.map(std::io::Cursor::new);
                        match Cartridge::load(&mut rom, wram, bios.as_mut(), file_name) {
                            Ok(cart) => {
                                let cart = if game_genie {
                                    cart.with_game_genie()
                                } else {
                                    cart
                                };
                                let cart_info = match cart.info() {
                                    nes::CartridgeInfo::Cartridge => CartridgeKind::Cartridge,
                                    nes::CartridgeInfo::Fds { total_sides } => CartridgeKind::Fds {
                                        current_side: Some(0),
                                        total_sides,
                                    },
                                };
                                self.save_store.clear();
                                self.frame = 0;
                                self.cart_id = Some(cart_id);
                                self.movie_input = None;
                                let machine = Machine::new(region, cart);
                                machine.set_debug_interest(
                                    self.debug_request.interests.iter().copied(),
                                );
                                self.machine = Some(machine);
                                self.blip
                                    .set_rates(region.cpu_clock(), self.sample_rate as f64);
                                max_step = region.frame_ticks().ceil() as u32;
                                samples_per_frame =
                                    (self.sample_rate as f64 / region.refresh_rate()).ceil()
                                        as usize;
                                self.commands.send_cartridge_info(cart_info);
                                self.input_source = SimpleInput::new();
                            }
                            Err(e) => tracing::error!("Unable to load cartridge: {e:?}"),
                        }
                    }
                    EmulatorInput::DebugRequest(req) => {
                        self.debug_request = req;
                        if let Some(machine) = self.machine.as_mut() {
                            machine
                                .set_debug_interest(self.debug_request.interests.iter().copied());
                        }
                    }
                    EmulatorInput::FastForward(toggle) => fast_forwarding = toggle,
                    EmulatorInput::SetFdsDisk(disk) => {
                        self.input_source
                            .handle_input(UserInput::Mapper(MapperInput::Fds(FdsInput::SetDisk(
                                disk,
                            ))));
                    }
                    EmulatorInput::SaveWram => {
                        if let Some((wram, cart_id)) = self
                            .machine
                            .as_ref()
                            .and_then(|m| m.save_wram())
                            .zip(self.cart_id.clone())
                        {
                            self.commands.send_wram(cart_id, wram);
                        }
                    }
                    EmulatorInput::PlayMovie(mut movie) => {
                        movie.prepare_frame();
                        self.movie_input = Some(movie);
                    }
                    EmulatorInput::ChannelPlayback(playback) => {
                        if let Some(machine) = self.machine.as_mut() {
                            machine.set_channel_playback(playback);
                        }
                    }
                    EmulatorInput::RecordWav(file) => {
                        if let Err(err) = self.samples_tx.start_recording(file, self.sample_rate) {
                            tracing::error!("recording wav: {err:?}");
                        }
                    }
                    EmulatorInput::StopRecordWav => {
                        if let Err(err) = self.samples_tx.end_recording() {
                            tracing::error!("recording wav: {err:?}");
                        }
                    }
                }
            }

            if fast_forwarding {
                playback = Playback::FastForward;
            }

            let wants_samples = if playback.skip_sleep() {
                self.samples_tx.wants_samples()
            } else if rewinding {
                if self.samples_tx.wants_sample_count(samples_per_frame) {
                    Some(samples_per_frame)
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                    None
                }
            } else {
                self.samples_tx
                    .wait_for_wants_samples(Duration::from_millis(4))
            };

            let Some(samples) = wants_samples else {
                continue;
            };

            if playback.skip_step() || self.debug.on_breakpoint() {
                continue;
            }

            let next_frame = run_until::Frames(1);

            if rewinding {
                if let Some((machine, (_frame, data))) =
                    self.machine.as_mut().zip(self.save_store.pop())
                {
                    machine.restore_state(&data);
                    self.frame = machine.frame() as usize;
                }

                // Rewinding requires frame sized steps so the full frame of audio can be
                // reversed at once
                self.step(Playback::Rewind, next_frame);
            } else {
                let mut clocks = self.blip.clocks_needed(samples as u32);
                while clocks > max_step {
                    self.step(playback, run_until::Samples(max_step).or(next_frame));
                    clocks -= max_step;
                }

                self.step(playback, run_until::Samples(clocks).or(next_frame));
            }
        }
    }

    fn step<U: RunUntil>(&mut self, playback: Playback, until: U) {
        if let Some(machine) = self.machine.as_mut() {
            let break_handler = |debug: &nes::Debug| {
                let event_notif = debug.take_interest_notification();
                if event_notif & self.debug_request.interest_breakpoints != 0 {
                    return true;
                }

                let s = debug.machine_state();
                if let Some(addr) = s.cpu.instruction_addr {
                    self.debug_request.breakpoints.is_set(addr)
                } else {
                    false
                }
            };

            let run_result = if let Some(movie) = self.movie_input.as_mut() {
                let res = machine.run_with_breakpoints(
                    nes::FrameEnd::SetVblank,
                    until,
                    break_handler,
                    movie,
                );
                if movie.done() {
                    self.movie_input = None;
                }
                res
            } else {
                machine.run_with_breakpoints(
                    nes::FrameEnd::SetVblank,
                    until,
                    break_handler,
                    &mut self.input_source,
                )
            };

            let frame = machine.frame() as usize;

            match run_result {
                RunResult::Done => {
                    if self.frame != frame {
                        if let Some(movie) = self.movie_input.as_mut() {
                            movie.prepare_frame();
                        }
                        self.frame = frame;

                        if playback.save_state() {
                            self.save_store.push(self.frame, || machine.save_state());
                        }

                        self.total_frames += 1;
                        if self.frame % playback.frame_freq() == 0 {
                            self.update_frame();
                        }
                        self.update_debug(playback.update_debug());
                    } else if playback.update_debug() {
                        self.update_frame();
                        self.update_debug(true);
                    }
                    self.update_audio(playback);
                }
                RunResult::Breakpoint => {
                    self.debug.set_breakpoint();
                    self.update_frame();
                    self.update_debug(true);
                }
            }
        }
    }

    fn update_audio(&mut self, playback: Playback) {
        if !playback.update_audio() {
            return;
        }

        if let Some(machine) = self.machine.as_mut() {
            let samples = machine.take_samples();
            let count = samples.len();
            if playback.reverse() {
                for (i, v) in samples.rev().enumerate() {
                    self.blip.add_delta(i as u32, v as i32 - self.blip_delta);
                    self.blip_delta = v as i32;
                }
            } else {
                for (i, v) in samples.enumerate() {
                    self.blip.add_delta(i as u32, v as i32 - self.blip_delta);
                    self.blip_delta = v as i32;
                }
            }
            self.blip.end_frame(count as u32);
            self.samples_tx.add_samples_from_blip(&mut self.blip);
        }
    }

    fn update_frame(&mut self) {
        if let Some(machine) = self.machine.as_mut() {
            self.back_buffer.update(|frame| {
                frame.copy_from_slice(machine.get_screen());
            });
        }
    }

    fn update_debug(&mut self, force_update: bool) {
        if !force_update
            && (self.debug_request.interval == 0
                || self.total_frames % self.debug_request.interval != 0)
        {
            return;
        }
        if let Some(machine) = self.machine.as_mut() {
            if self.debug_request.cpu_mem {
                self.debug.cpu_mem.update(|data| {
                    for (addr, v) in data.iter_mut().enumerate() {
                        *v = machine.peek(addr as u16);
                    }
                });
            }

            if self.debug_request.ppu_mem {
                self.debug.ppu_mem.update(|data| {
                    for (addr, v) in data.iter_mut().enumerate() {
                        *v = machine.peek_ppu(addr as u16);
                    }
                })
            }

            let debug = machine.get_debug();
            if self.debug_request.pal_ram {
                self.debug.pal_ram.update(|data| {
                    data.copy_from_slice(debug.pallete_ram(machine));
                })
            }

            if self.debug_request.sprite_ram {
                self.debug.sprite_ram.update(|data| {
                    data.copy_from_slice(debug.sprite_ram(machine));
                })
            }

            if self.debug_request.state {
                self.debug.state.update(|data| {
                    *data = debug.machine_state();
                });
            }

            if self.debug_request.events {
                self.debug.events.update(|data| {
                    machine.read_debug_events(|events| data.copy_from_slice(events));
                });
            }

            if self.debug_request.frame {
                self.debug.frame.update(|data| {
                    data.copy_from_slice(machine.get_screen());
                });
            }

            if self.debug_request.channels {
                self.debug.channels.update(|data| {
                    data.clear();
                    data.extend(machine.take_channel_samples());
                });
            } else {
                // reset buffer to ensure clean samples next time they are viewed
                let _ = machine.take_channel_samples();
            }

            if self.debug_request.variables {
                self.debug.watch_items.update(|data| {
                    data.clear();
                    data.extend(machine.get_debug().watch_items());
                });
            }

            if self.debug_request.inputs {
                let input = if let Some(movie) = self.movie_input.as_ref() {
                    movie.peek()
                } else {
                    self.input_source.peek()
                };
                self.debug.inputs.update(|data| {
                    data[0] = input.0;
                    data[1] = input.1;
                });
            }

            self.debug.update();
        }
    }
}

struct SaveStoreBuilder {
    divisor: usize,
    generations: Vec<SaveStoreGeneration>,
}

impl SaveStoreBuilder {
    fn add(mut self, divisor: usize, capacity: usize) -> Self {
        self.divisor *= divisor;
        self.generations
            .push(SaveStoreGeneration::new(capacity, self.divisor));
        self
    }

    fn build(self) -> SaveStore {
        SaveStore {
            generations: self.generations,
        }
    }
}

struct SaveStore {
    generations: Vec<SaveStoreGeneration>,
}

impl SaveStore {
    fn builder() -> SaveStoreBuilder {
        SaveStoreBuilder {
            divisor: 1,
            generations: Vec::new(),
        }
    }

    fn pop(&mut self) -> Option<(usize, nes::SaveData)> {
        for save_gen in self.generations.iter_mut() {
            if let Some(state) = save_gen.pop() {
                return Some(state);
            }
        }

        None
    }

    fn push<F: FnOnce() -> nes::SaveData>(&mut self, frame: usize, func: F) {
        let mut carry_over = None;
        let mut func = Some(func);
        for save_gen in self.generations.iter_mut() {
            if let Some(func) = func.take() {
                carry_over = save_gen.push(frame, func);
            } else if let Some((frame, data)) = carry_over {
                carry_over = save_gen.push(frame, || data);
            }

            if carry_over.is_none() {
                break;
            }
        }
    }

    fn clear(&mut self) {
        for save_gen in self.generations.iter_mut() {
            save_gen.clear();
        }
    }
}

struct SaveStoreGeneration {
    capacity: usize,
    divisor: usize,
    saves: VecDeque<(usize, nes::SaveData)>,
}

impl SaveStoreGeneration {
    fn new(capacity: usize, divisor: usize) -> Self {
        Self {
            capacity,
            divisor,
            saves: VecDeque::with_capacity(capacity),
        }
    }

    fn pop(&mut self) -> Option<(usize, nes::SaveData)> {
        self.saves.pop_back()
    }

    fn push<F: FnOnce() -> nes::SaveData>(
        &mut self,
        frame: usize,
        func: F,
    ) -> Option<(usize, nes::SaveData)> {
        if frame % self.divisor != 0 {
            return None;
        }

        let data = func();

        let excess = if self.saves.len() == self.capacity {
            self.saves.pop_front()
        } else {
            None
        };

        self.saves.push_back((frame, data));

        excess
    }

    fn clear(&mut self) {
        self.saves.clear();
    }
}
