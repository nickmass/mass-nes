use std::{collections::VecDeque, time::Duration};

use blip_buf_rs::Blip;
use nes::{
    Cartridge, DebugEvent, FdsInput, Machine, MapperInput, Region, RunResult, SaveWram, UserInput,
    run_until::{self, RunUntil},
};
use tracing::instrument;
use ui::{audio::SamplesSender, movie::MovieFile, wram::CartridgeId};

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
    ),
    DebugRequest(DebugRequest),
    StepBack,
    StepForward,
    FastForward(bool),
    SetFdsDisk(Option<usize>),
    SaveWram,
    PlayMovie(MovieFile),
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
    blip: Blip,
    blip_delta: i32,
    save_states: Vec<Option<(usize, nes::SaveData)>>,
    save_store: SaveStore,
    frame: usize,
    total_frames: u64,
    debug: DebugSwapState,
    debug_request: DebugRequest,
}

impl Runner {
    pub fn new(
        commands: EmulatorCommands,
        back_buffer: GfxBackBuffer,
        samples_tx: SamplesSender,
        sample_rate: u32,
        debug: DebugSwapState,
    ) -> Self {
        let blip = blip_buf_rs::Blip::new(sample_rate);

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
            },
            movie_input: None,
        }
    }

    pub fn run(mut self) {
        let mut rewinding = false;
        let mut fast_forwarding = false;
        let mut max_step = nes::Region::Ntsc.frame_ticks().ceil() as u32;
        let mut samples_per_frame =
            (self.sample_rate as f64 / nes::Region::Ntsc.refresh_rate()).ceil() as usize;
        loop {
            let mut playback = Playback::Normal;
            while let Some(input) = self.commands.try_command() {
                match input {
                    EmulatorInput::Nes(input) => {
                        if let Some(machine) = &mut self.machine {
                            if self.movie_input.is_none() {
                                machine.handle_input(input)
                            }
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
                    EmulatorInput::StepForward => {
                        playback = Playback::StepForward;
                        self.step(playback, run_until::Frames(1));
                    }
                    EmulatorInput::LoadCartridge(cart_id, region, rom, file_name, wram, bios) => {
                        let mut rom = std::io::Cursor::new(rom);
                        let mut bios = bios.map(std::io::Cursor::new);
                        match Cartridge::load(&mut rom, wram, bios.as_mut(), file_name) {
                            Ok(cart) => {
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
                                self.blip.set_rates(
                                    region.frame_ticks() * region.refresh_rate(),
                                    self.sample_rate as f64,
                                );
                                max_step = region.frame_ticks().ceil() as u32;
                                samples_per_frame =
                                    (self.sample_rate as f64 / region.refresh_rate()).ceil()
                                        as usize;
                                self.commands.send_cartridge_info(cart_info);
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
                        if let Some(machine) = self.machine.as_mut() {
                            machine.handle_input(UserInput::Mapper(MapperInput::Fds(
                                FdsInput::SetDisk(disk),
                            )));
                        }
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
                    EmulatorInput::PlayMovie(movie) => {
                        self.movie_input = Some(movie);
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
                    .wait_for_wants_samples(Duration::from_millis(1))
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

    #[instrument(skip_all)]
    fn step<U: RunUntil>(&mut self, playback: Playback, until: U) {
        if let Some(machine) = self.machine.as_mut() {
            let frame_end = if self.movie_input.is_some() {
                nes::FrameEnd::SetVblank
            } else {
                nes::FrameEnd::ClearVblank
            };

            let run_result =
                machine.run_with_breakpoints(frame_end, until, |debug: &nes::Debug| {
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
                });

            let frame = machine.frame() as usize;

            match run_result {
                RunResult::Done => {
                    if self.frame != frame {
                        while let Some(input) = self.movie_input.as_mut().map(|fm2| fm2.next()) {
                            match input {
                                None => self.movie_input = None,
                                Some(ui::movie::MovieInput::Frame) => break,
                                Some(ui::movie::MovieInput::Input(input)) => {
                                    machine.handle_input(input)
                                }
                            }
                        }

                        self.frame = frame;

                        if playback.save_state() {
                            self.save_store.push(self.frame, || machine.save_state());
                        }

                        self.total_frames += 1;
                        if self.frame % playback.frame_freq() == 0 {
                            self.update_frame();
                        }
                        self.update_debug(false);
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    fn update_frame(&mut self) {
        if let Some(machine) = self.machine.as_mut() {
            self.back_buffer.update(|frame| {
                frame.copy_from_slice(machine.get_screen());
            });
        }
    }

    #[instrument(skip_all)]
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

            self.debug.update_at(self.total_frames);
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
