use std::{collections::VecDeque, time::Duration};

use blip_buf_rs::Blip;
use nes::{Cartridge, Machine, MachineState, Region, RunResult, UserInput};
use tracing::instrument;
use ui::audio::SamplesSender;

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
    LoadCartridge(Region, Vec<u8>),
    DebugRequest(DebugRequest),
    StepBack,
    StepForward,
    FastForward,
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
}

use crate::{
    app::EmulatorCommands, debug_state::DebugSwapState, gfx::GfxBackBuffer, widgets::Breakpoints,
};

pub struct Runner {
    machine: Option<Machine>,
    back_buffer: GfxBackBuffer,
    commands: Option<EmulatorCommands>,
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
        let blip = blip_buf_rs::Blip::new(sample_rate / 20);

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
            back_buffer,
            commands: Some(commands),
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
                breakpoints: Breakpoints::new(),
            },
        }
    }

    pub fn run(mut self) {
        let Some(mut commands) = self.commands.take() else {
            panic!("nes commands taken");
        };

        let mut rewinding = false;
        loop {
            let mut playback = Playback::Normal;
            for input in commands.try_commands() {
                match input {
                    EmulatorInput::Nes(input) => {
                        if let Some(machine) = &mut self.machine {
                            machine.handle_input(input)
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
                            if let Some((frame, data)) = self.save_states[slot as usize].as_ref() {
                                self.frame = *frame;
                                machine.restore_state(data);
                            }
                        }
                    }
                    EmulatorInput::Rewind(toggle) => {
                        rewinding = toggle;
                    }
                    EmulatorInput::StepBack => {
                        if let Some(machine) = &mut self.machine {
                            if let Some((frame, data)) = self.save_store.pop() {
                                self.frame = frame;
                                machine.restore_state(&data);
                                playback = Playback::StepBackward;
                                self.step(playback);
                            }
                        }
                    }
                    EmulatorInput::StepForward => {
                        playback = Playback::StepForward;
                        self.step(playback);
                    }
                    EmulatorInput::LoadCartridge(region, rom) => {
                        let mut rom = std::io::Cursor::new(rom);
                        if let Ok(cart) = Cartridge::load(&mut rom) {
                            self.save_store.clear();
                            self.frame = 0;
                            self.machine = Some(Machine::new(region, cart));
                            self.blip.set_rates(
                                region.frame_ticks() * region.refresh_rate(),
                                self.sample_rate as f64,
                            );
                        }
                    }
                    EmulatorInput::DebugRequest(req) => {
                        self.debug_request = req;
                    }
                    EmulatorInput::FastForward => {
                        playback = Playback::FastForward;
                    }
                }
            }

            if !playback.skip_step() && self.samples_tx.wants_samples() {
                if rewinding {
                    if let Some((machine, (frame, data))) =
                        self.machine.as_mut().zip(self.save_store.pop())
                    {
                        self.frame = frame;
                        machine.restore_state(&data);
                        playback = Playback::Rewind;
                    }
                }
                self.step(playback);
            }

            if !playback.skip_sleep() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    #[instrument(skip_all)]
    fn step(&mut self, playback: Playback) {
        if let Some(machine) = self.machine.as_mut() {
            if playback.save_state() {
                self.save_store.push(self.frame, || machine.save_state());
            }
            match machine.run_with_breakpoints(|s: &MachineState| {
                if let Some(addr) = s.cpu.instruction_addr {
                    self.debug_request.breakpoints.is_set(addr)
                } else {
                    false
                }
            }) {
                RunResult::Frame => {
                    self.frame += 1;
                    self.total_frames += 1;

                    if playback.update_audio() {
                        self.update_audio(playback);
                    }
                    if self.frame % playback.frame_freq() == 0 || true {
                        self.update_frame();
                    }
                    self.update_debug(false);
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
        for gen in self.generations.iter_mut() {
            if let Some(state) = gen.pop() {
                return Some(state);
            }
        }

        None
    }

    fn push<F: FnOnce() -> nes::SaveData>(&mut self, frame: usize, func: F) {
        let mut carry_over = None;
        let mut func = Some(func);
        for gen in self.generations.iter_mut() {
            if let Some(func) = func.take() {
                carry_over = gen.push(frame, func);
            } else if let Some((frame, data)) = carry_over {
                carry_over = gen.push(frame, || data);
            }

            if carry_over.is_none() {
                break;
            }
        }
    }

    fn clear(&mut self) {
        for gen in self.generations.iter_mut() {
            gen.clear();
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
