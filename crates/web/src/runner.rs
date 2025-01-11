use futures::StreamExt;
use web_sys::{
    js_sys::Array,
    wasm_bindgen::{self, prelude::*},
};

use ui::audio::SamplesProducer;

use crate::{
    app::{EmulatorInput, NesInputs},
    gfx::GfxBackBuffer,
    worker::WorkerSpawn,
};

pub struct MachineSpawner {
    pub region: nes::Region,
    pub sample_rate: u32,
    back_buffer: GfxBackBuffer,
    samples_producer: SamplesProducer,
    nes_inputs: NesInputs,
}

impl MachineSpawner {
    pub fn new(
        region: nes::Region,
        sample_rate: u32,
        back_buffer: GfxBackBuffer,
        samples_producer: SamplesProducer,
        nes_inputs: NesInputs,
    ) -> Self {
        Self {
            region,
            sample_rate,
            back_buffer,
            samples_producer,
            nes_inputs,
        }
    }
}

#[wasm_bindgen]
pub async fn machine_worker(ptr: u32, transferables: Array) {
    crate::worker::worker::<MachineSpawner>(ptr, transferables).await
}

impl WorkerSpawn for MachineSpawner {
    const ENTRY_POINT: &'static str = stringify!(machine_worker);

    async fn run(self, _transferables: Array) {
        let runner = MachineRunner::new(self);
        runner.run().await;
    }
}

struct MachineRunner {
    machine: Option<nes::Machine>,
    region: nes::Region,
    blip_delta: i32,
    blip: blip_buf_rs::Blip,
    back_buffer: GfxBackBuffer,
    audio_buffer: Vec<i16>,
    samples_producer: SamplesProducer,
    nes_inputs: Option<NesInputs>,
}

impl MachineRunner {
    fn new(channel: MachineSpawner) -> Self {
        let MachineSpawner {
            region,
            sample_rate,
            nes_inputs,
            back_buffer,
            samples_producer,
        } = channel;

        let mut blip = blip_buf_rs::Blip::new(sample_rate / 30);
        blip.set_rates(
            region.frame_ticks() * region.refresh_rate(),
            sample_rate as f64,
        );

        Self {
            machine: None,
            region,
            nes_inputs: Some(nes_inputs),
            blip_delta: 0,
            blip,
            back_buffer,
            audio_buffer: vec![0; 1024],
            samples_producer,
        }
    }

    pub async fn run(mut self) {
        let Some(inputs) = self.nes_inputs.take() else {
            panic!("no machine_channel inputs");
        };

        let mut inputs = inputs.inputs();

        while let Some(input) = inputs.next().await {
            self.handle_input(input).await;
        }
    }

    async fn handle_input(&mut self, input: EmulatorInput) {
        match input {
            EmulatorInput::Load(rom) => {
                let mut rom = std::io::Cursor::new(rom);
                let Ok(cart) = nes::Cartridge::load(&mut rom) else {
                    tracing::error!("failed to load rom");
                    return;
                };
                let machine = nes::Machine::new(self.region, cart);
                self.machine = Some(machine);
            }
            EmulatorInput::UserInput(input) => {
                if let Some(machine) = self.machine.as_mut() {
                    machine.handle_input(input);
                    machine.run();

                    let samples = machine.get_audio();
                    let count = samples.len();

                    for (i, v) in samples.iter().enumerate() {
                        self.blip.add_delta(i as u32, *v as i32 - self.blip_delta);
                        self.blip_delta = *v as i32;
                    }
                    self.blip.end_frame(count as u32);

                    while self.blip.samples_avail() > 0 {
                        let count =
                            self.blip.read_samples(&mut self.audio_buffer, 1024, false) as usize;
                        self.samples_producer
                            .add_samples(&self.audio_buffer[..count]);
                    }

                    self.back_buffer
                        .update(|frame| {
                            frame.copy_from_slice(machine.get_screen());
                        })
                        .await;
                }
            }
        }
    }
}
