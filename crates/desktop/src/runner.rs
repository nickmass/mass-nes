use std::collections::VecDeque;

use blip_buf_rs::Blip;
use nes::{Cartridge, Machine, Region, UserInput};
use tracing::instrument;
use ui::audio::SamplesProducer;

use crate::{
    app::{EmulatorInput, NesInputs},
    gfx::GfxBackBuffer,
    TracyExt,
};

pub struct Runner {
    machine: Machine,
    back_buffer: GfxBackBuffer,
    inputs: Option<NesInputs>,
    samples_producer: Option<SamplesProducer>,
    blip: Blip,
    blip_delta: i32,
    audio_buffer: Vec<i16>,
    save_states: Vec<Option<(usize, nes::SaveData)>>,
    save_store: SaveStore,
    frame: usize,
}

impl Runner {
    pub fn new(
        cart: Cartridge,
        region: Region,
        inputs: NesInputs,
        back_buffer: GfxBackBuffer,
        samples_producer: Option<SamplesProducer>,
        sample_rate: u32,
    ) -> Self {
        let machine = instrument_machine(Machine::new(region, cart));
        let mut blip = blip_buf_rs::Blip::new(sample_rate / 30);
        blip.set_rates(
            region.frame_ticks() * region.refresh_rate(),
            sample_rate as f64,
        );

        Self {
            machine,
            back_buffer,
            inputs: Some(inputs),
            samples_producer,
            blip,
            blip_delta: 0,
            audio_buffer: vec![0; 1024],
            save_states: vec![None; 10],
            save_store: SaveStore::new(32000, 5),
            frame: 0,
        }
    }

    pub fn run(mut self) {
        let Some(inputs) = self.inputs.take() else {
            panic!("nes inputs taken");
        };

        for input in inputs.inputs() {
            match input {
                EmulatorInput::Nes(input) => self.handle_input(input),
                EmulatorInput::SaveState(slot) => {
                    let data = self.machine.save_state();

                    self.save_states[slot as usize] = Some((self.frame, data));
                }
                EmulatorInput::RestoreState(slot) => {
                    if let Some((frame, data)) = self.save_states[slot as usize].as_ref() {
                        self.frame = *frame;
                        self.machine.restore_state(data);
                    }
                }
                EmulatorInput::Rewind => {
                    if let Some((frame, data)) = self.save_store.pop() {
                        self.frame = frame;
                        self.machine.restore_state(&data);
                    }
                }
            }
        }
    }

    fn handle_input(&mut self, input: UserInput) {
        self.machine.handle_input(input);
        self.machine.run();

        self.frame += 1;
        self.save_store
            .push(self.frame, || self.machine.save_state());

        self.update_audio();
        self.update_frame();
    }

    #[instrument(skip_all)]
    fn update_audio(&mut self) {
        if let Some(samples_producer) = self.samples_producer.as_mut() {
            let samples = self.machine.get_audio();
            let count = samples.len();

            for (i, v) in samples.iter().enumerate() {
                self.blip.add_delta(i as u32, *v as i32 - self.blip_delta);
                self.blip_delta = *v as i32;
            }
            self.blip.end_frame(count as u32);
            while self.blip.samples_avail() > 0 {
                let count = self.blip.read_samples(&mut self.audio_buffer, 1024, false) as usize;
                samples_producer.add_samples(&self.audio_buffer[..count]);
            }
        }
    }

    #[instrument(skip_all)]
    fn update_frame(&mut self) {
        self.back_buffer.update(|frame| {
            frame.copy_from_slice(self.machine.get_screen());
        });
    }
}

fn instrument_machine(machine: Machine) -> Machine {
    if let Some(client) = tracy_client::Client::running() {
        client.plot_config(c"scanline", true, true, None);
        client.plot_config(c"vblank", true, true, None);
        client.plot_config(c"nmi", true, true, None);
    }
    let mut scanline = 0;
    let mut vblank = false;
    let mut nmi = false;
    machine.with_trace_fn(move |_cpu, ppu| {
        if let Some(client) = tracy_client::Client::running() {
            if scanline != ppu.scanline {
                client.plot_int(c"scanline", ppu.scanline as i64);
                scanline = ppu.scanline;
            }
            if vblank != ppu.vblank {
                client.plot_int(c"vblank", ppu.vblank as i64);
                vblank = ppu.vblank;
            }
            if nmi != ppu.nmi {
                client.plot_int(c"nmi", ppu.nmi as i64);
                nmi = ppu.nmi;
            }
        }
    })
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
