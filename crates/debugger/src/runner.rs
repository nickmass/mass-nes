use std::{collections::VecDeque, time::Duration};

use blip_buf_rs::Blip;
use nes::{Cartridge, Machine, MachineState, Region, RunResult, UserInput};
use tracing::instrument;
use ui::audio::SamplesSender;

#[derive(Debug)]
#[allow(unused)]
pub enum EmulatorInput {
    Nes(UserInput),
    Rewind,
    SaveState(u32),
    RestoreState(u32),
    LoadCartridge(Region, Vec<u8>),
    DebugRequest(DebugRequest),
    StepBack,
    StepForward,
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
    audio_buffer: Vec<i16>,
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
        let blip = blip_buf_rs::Blip::new(sample_rate / 30);

        Self {
            machine: None,
            back_buffer,
            commands: Some(commands),
            samples_tx,
            sample_rate,
            blip,
            blip_delta: 0,
            audio_buffer: vec![0; 1024],
            save_states: vec![None; 10],
            save_store: SaveStore::new(32000, 5),
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

        loop {
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
                    EmulatorInput::Rewind => {
                        if let Some(machine) = &mut self.machine {
                            if let Some((frame, data)) = self.save_store.pop() {
                                self.frame = frame;
                                machine.restore_state(&data);
                            }
                        }
                    }
                    EmulatorInput::StepBack => {
                        if let Some(machine) = &mut self.machine {
                            if let Some((frame, data)) = self.save_store.pop() {
                                self.frame = frame;
                                machine.restore_state(&data);
                            }
                        }
                        self.step();
                    }
                    EmulatorInput::StepForward => {
                        self.step();
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
                }
            }

            if self.samples_tx.wants_samples() {
                self.step();
            }

            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[instrument(skip_all)]
    fn step(&mut self) {
        if let Some(machine) = self.machine.as_mut() {
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
                    self.save_store.push(self.frame, || machine.save_state());

                    self.update_audio();
                    self.update_frame();
                    self.update_debug(false);
                }
                RunResult::Breakpoint => {
                    let _ = machine.get_audio();
                    self.debug.set_breakpoint();
                    self.update_frame();
                    self.update_debug(true);
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn update_audio(&mut self) {
        if let Some(machine) = self.machine.as_mut() {
            let samples = machine.get_audio();
            let count = samples.len();

            for (i, v) in samples.iter().enumerate() {
                self.blip.add_delta(i as u32, *v as i32 - self.blip_delta);
                self.blip_delta = *v as i32;
            }
            self.blip.end_frame(count as u32);
            while self.blip.samples_avail() > 0 {
                let count = self.blip.read_samples(&mut self.audio_buffer, 1024, false) as usize;
                self.samples_tx.add_samples(&self.audio_buffer[..count]);
            }
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

struct SaveStore {
    limit: usize,
    freq: usize,
    saves: VecDeque<(usize, nes::SaveData)>,
}

impl SaveStore {
    fn new(limit: usize, freq: usize) -> Self {
        Self {
            limit,
            freq,
            saves: VecDeque::new(),
        }
    }

    fn pop(&mut self) -> Option<(usize, nes::SaveData)> {
        self.saves.pop_back()
    }

    fn push<F: FnOnce() -> nes::SaveData>(&mut self, frame: usize, func: F) {
        if frame % self.freq != 0 {
            return;
        }

        let data = func();

        if self.saves.len() == self.limit {
            self.saves.pop_front();
        }

        self.saves.push_back((frame, data));
    }

    fn clear(&mut self) {
        self.saves.clear();
    }
}
