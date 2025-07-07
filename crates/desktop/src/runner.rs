use std::{collections::VecDeque, time::Duration};

use blip_buf::BlipBuf;
use nes::{
    Cartridge, FrameEnd, Machine, Region, UserInput,
    run_until::{self, RunUntil},
};
use ui::audio::SamplesSender;

use crate::{
    app::{EmulatorInput, NesInputs},
    gfx::GfxBackBuffer,
};

pub struct Runner {
    machine: Machine,
    back_buffer: GfxBackBuffer,
    inputs: Option<NesInputs>,
    samples_tx: SamplesSender,
    blip: BlipBuf,
    blip_delta: i32,
    save_states: Vec<Option<(usize, nes::SaveData)>>,
    save_store: SaveStore,
    frame: Option<u32>,
}

impl Runner {
    pub fn new(
        cart: Cartridge,
        region: Region,
        inputs: NesInputs,
        back_buffer: GfxBackBuffer,
        samples_tx: SamplesSender,
        sample_rate: u32,
    ) -> Self {
        let machine = Machine::new(region, cart);
        let mut blip = BlipBuf::new(sample_rate / 20);
        blip.set_rates(region.cpu_clock(), sample_rate as f64);

        Self {
            machine,
            back_buffer,
            inputs: Some(inputs),
            samples_tx,
            blip,
            blip_delta: 0,
            save_states: vec![None; 10],
            save_store: SaveStore::new(32000, 5),
            frame: None,
        }
    }

    pub fn run(mut self) {
        let Some(mut inputs) = self.inputs.take() else {
            panic!("nes inputs taken");
        };

        loop {
            for input in inputs.try_inputs() {
                match input {
                    EmulatorInput::Nes(input) => self.handle_input(input),
                    EmulatorInput::SaveState(slot) => {
                        if let Some(frame) = self.frame {
                            let data = self.machine.save_state();

                            self.save_states[slot as usize] = Some((frame as usize, data));
                        }
                    }
                    EmulatorInput::RestoreState(slot) => {
                        if let Some((frame, data)) = self.save_states[slot as usize].as_ref() {
                            self.frame = Some(*frame as u32);
                            self.machine.restore_state(data);
                        }
                    }
                    EmulatorInput::Rewind => {
                        if let Some((frame, data)) = self.save_store.pop() {
                            self.frame = Some(frame as u32);
                            self.machine.restore_state(&data);
                        }
                    }
                }
            }

            if let Some(samples) = self
                .samples_tx
                .wait_for_wants_samples(Duration::from_millis(1))
            {
                self.step(samples as u32);
            }
        }
    }

    fn handle_input(&mut self, input: UserInput) {
        self.machine.handle_input(input);
    }

    fn step(&mut self, samples: u32) {
        self.machine.run_with_breakpoints(
            FrameEnd::SetVblank,
            run_until::Frames(1).or(run_until::Samples(samples)),
            (),
        );

        self.update_audio();
        let frame = self.machine.frame();
        if self.frame != Some(frame) {
            self.frame = Some(frame);
            self.save_store
                .push(frame as usize, || self.machine.save_state());
            self.update_frame();
        }
    }

    fn update_audio(&mut self) {
        let mut count = 0;
        for (i, v) in self.machine.get_samples().enumerate() {
            self.blip.add_delta(i as u32, v as i32 - self.blip_delta);
            self.blip_delta = v as i32;
            count += 1;
        }
        self.blip.end_frame(count as u32);
        self.samples_tx.add_samples_from_blip(&mut self.blip);
    }

    fn update_frame(&mut self) {
        self.back_buffer.update(|frame| {
            frame.copy_from_slice(self.machine.get_screen());
        });
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
}
