use std::time::Duration;

use nes::{FrameEnd, run_until::RunUntil};
use web_sys::{
    js_sys::Array,
    wasm_bindgen::{self, prelude::*},
};
use web_worker::WorkerSpawn;

use ui::audio::SamplesSender;

use crate::{
    app::{EmulatorInput, NesInputs},
    gfx::GfxBackBuffer,
};

pub struct MachineSpawner {
    pub region: nes::Region,
    pub sample_rate: u32,
    back_buffer: GfxBackBuffer,
    samples_tx: SamplesSender,
    nes_inputs: NesInputs,
}

impl MachineSpawner {
    pub fn new(
        region: nes::Region,
        sample_rate: u32,
        back_buffer: GfxBackBuffer,
        samples_tx: SamplesSender,
        nes_inputs: NesInputs,
    ) -> Self {
        Self {
            region,
            sample_rate,
            back_buffer,
            samples_tx,
            nes_inputs,
        }
    }
}

#[wasm_bindgen]
pub async fn machine_worker(ptr: u32, transferables: Array) {
    web_worker::worker::<MachineSpawner>(ptr, transferables).await
}

impl WorkerSpawn for MachineSpawner {
    const ENTRY_POINT: &'static str = stringify!(machine_worker);

    async fn run(self, _transferables: Array) {
        let runner = MachineRunner::new(self);
        runner.run();
    }
}

struct MachineRunner {
    machine: Option<nes::Machine>,
    region: nes::Region,
    blip_delta: i32,
    blip: blip_buf::BlipBuf,
    back_buffer: GfxBackBuffer,
    samples_tx: SamplesSender,
    nes_inputs: Option<NesInputs>,
    last_frame: Option<u32>,
}

impl MachineRunner {
    fn new(channel: MachineSpawner) -> Self {
        let MachineSpawner {
            region,
            sample_rate,
            nes_inputs,
            back_buffer,
            samples_tx,
        } = channel;

        let mut blip = blip_buf::BlipBuf::new(sample_rate / 20);
        blip.set_rates(region.cpu_clock(), sample_rate as f64);

        Self {
            machine: None,
            region,
            nes_inputs: Some(nes_inputs),
            blip_delta: 0,
            blip,
            back_buffer,
            samples_tx,
            last_frame: None,
        }
    }

    pub fn run(mut self) {
        let Some(mut inputs) = self.nes_inputs.take() else {
            panic!("no machine_channel inputs");
        };

        while self.machine.is_none() {
            if let Some(input) = inputs.next() {
                self.handle_input(input);
            }
        }

        loop {
            for input in inputs.try_recv() {
                self.handle_input(input);
            }

            if let Some(samples) = self
                .samples_tx
                .wait_for_wants_samples(Duration::from_millis(1))
            {
                self.step(samples as u32);
            }
        }
    }

    fn handle_input(&mut self, input: EmulatorInput) {
        match input {
            EmulatorInput::Load(rom) => {
                let mut rom = std::io::Cursor::new(rom);
                let Ok(cart) = nes::Cartridge::load(&mut rom, None, None, "rom.nes") else {
                    tracing::error!("failed to load rom");
                    return;
                };
                let machine = nes::Machine::new(self.region, cart);
                self.machine = Some(machine);
                self.last_frame = None;
            }
            EmulatorInput::UserInput(input) => {
                if let Some(machine) = self.machine.as_mut() {
                    machine.handle_input(input);
                }
            }
        }
    }

    fn step(&mut self, samples: u32) {
        if let Some(machine) = self.machine.as_mut() {
            let until = nes::run_until::Frames(1).or(nes::run_until::Samples(samples));
            machine.run_with_breakpoints(FrameEnd::SetVblank, until, ());
            let frame = Some(machine.frame());

            if frame != self.last_frame {
                self.back_buffer.update(|frame| {
                    frame.copy_from_slice(machine.get_screen());
                });
                self.last_frame = frame;
            }

            let mut count = 0;

            for (i, v) in machine.get_samples().enumerate() {
                self.blip.add_delta(i as u32, v as i32 - self.blip_delta);
                self.blip_delta = v as i32;
                count += 1;
            }
            self.blip.end_frame(count as u32);

            self.samples_tx.add_samples_from_blip(&mut self.blip);
        }
    }
}
