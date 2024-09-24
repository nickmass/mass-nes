use futures::StreamExt;
use web_sys::{
    js_sys::{self, Object},
    wasm_bindgen::{self, closure::Closure, prelude::*, JsCast, JsValue},
    DedicatedWorkerGlobalScope, Worker, WorkerOptions,
};

use ui::audio::SamplesProducer;

use crate::{
    app::{EmulatorInput, NesInputs},
    gfx_worker::GfxBackBuffer,
};

#[wasm_bindgen]
pub async fn worker_machine(channel: u32) {
    let channel = unsafe { MachineWorkerChannel::from_raw(channel) };
    let global: DedicatedWorkerGlobalScope = js_sys::global().dyn_into().unwrap();
    global.post_message(&JsValue::TRUE).unwrap();

    let machine = MachineRunner::new(*channel);
    machine.run().await;
    tracing::debug!("machine exit");
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
    fn new(channel: MachineWorkerChannel) -> Self {
        let MachineWorkerChannel {
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
                            for (a, b) in frame.iter_mut().zip(machine.get_screen()) {
                                *a = b.get();
                            }
                        })
                        .await;
                }
            }
        }
    }
}

pub struct MachineWorker;

impl MachineWorker {
    pub async fn new(
        nes_inputs: NesInputs,
        back_buffer: GfxBackBuffer,
        samples_producer: SamplesProducer,
        region: nes::Region,
        sample_rate: u32,
    ) {
        let opts = WorkerOptions::new();
        opts.set_type(web_sys::WorkerType::Module);
        let worker = Worker::new_with_options("worker.js", &opts).unwrap();

        let (init_tx, init_rx) = futures::channel::oneshot::channel();
        let on_message = Closure::once_into_js(move || {
            init_tx.send(()).unwrap_throw();
        });
        worker.set_onmessage(Some(&on_message.as_ref().unchecked_ref()));

        let channel = MachineWorkerChannel {
            region,
            sample_rate,
            nes_inputs,
            back_buffer,
            samples_producer,
        };

        let init: JsValue = MachineWorkerInit::new(channel).into();
        worker.post_message(&init).unwrap();
        init_rx.await.unwrap();
    }
}

pub struct MachineWorkerChannel {
    pub region: nes::Region,
    pub sample_rate: u32,
    back_buffer: GfxBackBuffer,
    samples_producer: SamplesProducer,
    nes_inputs: NesInputs,
}

impl MachineWorkerChannel {
    unsafe fn from_raw(raw: u32) -> Box<Self> {
        Box::from_raw(raw as *mut _)
    }
}

struct MachineWorkerInit {
    memory: JsValue,
    module: JsValue,
    channel: u32,
}

impl MachineWorkerInit {
    fn new(channel: MachineWorkerChannel) -> Self {
        let channel = Box::new(channel);
        let channel = Box::into_raw(channel) as u32;
        Self {
            memory: wasm_bindgen::memory(),
            module: wasm_bindgen::module(),
            channel,
        }
    }
}

impl Into<JsValue> for MachineWorkerInit {
    fn into(self) -> JsValue {
        let obj = Object::new();
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("worker_type"),
            &JsValue::from_str("machine"),
        );
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("memory"), &self.memory);
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("module"), &self.module);
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("channel"),
            &JsValue::from_f64(self.channel as f64),
        );

        obj.into()
    }
}
