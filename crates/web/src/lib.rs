use futures::{SinkExt, Stream, StreamExt};
use gloo::{
    net::http,
    worker::{
        reactor::{reactor, ReactorBridge, ReactorScope},
        Registrable, Spawnable,
    },
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{js_sys, wasm_bindgen, HtmlCanvasElement, WorkerGlobalScope};
use winit::event_loop::EventLoopProxy;

use std::io::Cursor;

use nes::{Cartridge, Controller, Machine, Region, UserInput};
use ui::audio::Audio;

mod app;
mod audio;
mod gfx;
mod gl;
mod sync;

use app::{NesInputs, NesOutputs, UserEvent};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Region")]
enum RegionDef {
    Ntsc,
    Pal,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "UserInput")]
enum UserInputDef {
    PlayerOne(#[serde(with = "ControllerDef")] Controller),
    Power,
    Reset,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Controller")]
struct ControllerDef {
    a: bool,
    b: bool,
    select: bool,
    start: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[derive(Serialize, Deserialize)]
enum MachineInput {
    Init {
        #[serde(with = "RegionDef")]
        region: Region,
        sample_rate: u32,
    },
    Load(Vec<u8>),
    UserInput(#[serde(with = "UserInputDef")] UserInput),
}

#[derive(Serialize, Deserialize)]
enum MachineOutput {
    InitFailure(InitFailureCause),
    InitSuccess,
    InvalidMessage(MachineInput),
    AudioSamples(Vec<i16>),
    Frame(Vec<u16>),
}

#[derive(Serialize, Deserialize)]
enum InitFailureCause {
    InvalidCartridge,
    InvalidMessage(MachineInput),
    ReactorShutdown,
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::DEBUG)
            .build(),
    )
}

#[wasm_bindgen]
pub async fn worker() {
    MachineRunner::registrar().register();
}

pub fn global() -> WorkerGlobalScope {
    use wasm_bindgen::JsCast;

    js_sys::global()
        .dyn_into::<WorkerGlobalScope>()
        .ok()
        .unwrap()
}

#[reactor]
async fn MachineRunner(mut scope: ReactorScope<MachineInput, MachineOutput>) {
    let (region, sample_rate) = match scope.next().await {
        Some(MachineInput::Init {
            region,
            sample_rate,
        }) => (region, sample_rate),
        Some(m) => {
            let _ = scope
                .send(MachineOutput::InitFailure(
                    InitFailureCause::InvalidMessage(m),
                ))
                .await;
            return;
        }
        None => {
            let _ = scope
                .send(MachineOutput::InitFailure(
                    InitFailureCause::ReactorShutdown,
                ))
                .await;
            return;
        }
    };

    let _ = scope.send(MachineOutput::InitSuccess).await;

    machine_loop(scope, region, sample_rate).await
}

async fn machine_loop(
    mut scope: ReactorScope<MachineInput, MachineOutput>,
    region: Region,
    sample_rate: u32,
) {
    let mut delta = 0;
    let mut blip = blip_buf_rs::Blip::new(sample_rate / 30);
    blip.set_rates(
        region.frame_ticks() * region.refresh_rate(),
        sample_rate as f64,
    );
    let mut machine = None;

    while let Some(event) = scope.next().await {
        let input = match event {
            MachineInput::UserInput(input) => input,
            MachineInput::Load(rom) => {
                let mut rom = Cursor::new(rom);
                let Ok(cartridge) = Cartridge::load(&mut rom) else {
                    let _ = scope
                        .send(MachineOutput::InitFailure(
                            InitFailureCause::InvalidCartridge,
                        ))
                        .await;
                    return;
                };

                machine = Some(Machine::new(region, cartridge));
                continue;
            }
            m => {
                let _ = scope.send(MachineOutput::InvalidMessage(m)).await;
                continue;
            }
        };

        let Some(machine) = machine.as_mut() else {
            continue;
        };

        machine.handle_input(input);
        machine.run();

        let samples = machine.get_audio();
        let count = samples.len();

        for (i, v) in samples.iter().enumerate() {
            blip.add_delta(i as u32, *v as i32 - delta);
            delta = *v as i32;
        }
        blip.end_frame(count as u32);

        while blip.samples_avail() > 0 {
            let mut buf = vec![0i16; 1024];
            let count = blip.read_samples(&mut buf, 1024, false);
            buf.truncate(count as usize);
            let _ = scope.send(MachineOutput::AudioSamples(buf)).await;
        }

        let frame = machine.get_screen().iter().map(|p| p.get()).collect();
        let _ = scope.send(MachineOutput::Frame(frame)).await;
    }
}

async fn create_worker() -> ReactorBridge<MachineRunner> {
    let machine_bridge = MachineRunner::spawner()
        .as_module(true)
        .with_loader(true)
        .spawn("./worker.js");

    machine_bridge
}

async fn worker_input_proxy(
    mut machine_sink: impl SinkExt<MachineInput> + std::marker::Unpin,
    inputs: NesInputs,
) {
    let mut inputs = inputs.inputs();
    while let Some(i) = inputs.next().await {
        let msg = match i {
            app::EmulatorInput::UserInput(i) => MachineInput::UserInput(i),
            app::EmulatorInput::Load(rom) => MachineInput::Load(rom),
        };

        let _ = machine_sink.send(msg).await;
    }
}

async fn worker_output_proxy(
    mut machine_stream: impl Stream<Item = MachineOutput> + std::marker::Unpin,
    outputs: NesOutputs,
) {
    while let Some(m) = machine_stream.next().await {
        match m {
            MachineOutput::AudioSamples(samples) => outputs.send_samples(samples),
            MachineOutput::Frame(frame) => outputs.send_frame(frame),
            MachineOutput::InvalidMessage(_) => (),
            MachineOutput::InitFailure(_) => (),
            MachineOutput::InitSuccess => (),
        }
    }
}

#[wasm_bindgen]
pub struct Emulator {
    proxy: EventLoopProxy<UserEvent>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub async fn new(region: String, canvas: HtmlCanvasElement) -> Option<Emulator> {
        let region = region.to_lowercase();
        let region = match region.as_str() {
            "pal" => Region::Pal,
            "ntsc" | _ => Region::Ntsc,
        };

        let sync = audio::CpalSync::new();
        let (audio, sync) = ui::audio::CpalAudio::new(sync, region.refresh_rate(), 128).ok()?;

        let sample_rate = audio.sample_rate();

        let mut machine_runner = create_worker().await;
        machine_runner
            .send(MachineInput::Init {
                region,
                sample_rate,
            })
            .await
            .ok()?;
        match machine_runner.next().await? {
            MachineOutput::InitSuccess => (),
            _ => return None,
        }

        let setup = ui::filters::NesNtscSetup::composite();
        let filter = ui::filters::NtscFilter::new(&setup);
        //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());
        let mut app = app::App::new(filter, audio, sync, canvas);

        let (machine_sink, machine_stream) = machine_runner.split();
        let (nes_inputs, nes_outputs) = app.nes_io();
        spawn_local(worker_output_proxy(machine_stream, nes_outputs));
        spawn_local(worker_input_proxy(machine_sink, nes_inputs));

        let proxy = app.proxy();

        app.run();

        Some(Self { proxy })
    }

    fn load_rom_bytes(&self, bytes: Vec<u8>) {
        let _ = self.proxy.send_event(UserEvent::Load(bytes));
        let _ = self.proxy.send_event(UserEvent::Sync);
    }

    #[wasm_bindgen]
    pub fn load_rom_array_buffer(&self, buffer: js_sys::ArrayBuffer) {
        let buffer_u8 = js_sys::Uint8Array::new(&buffer);
        let bytes = buffer_u8.to_vec();
        self.load_rom_bytes(bytes);
    }

    #[wasm_bindgen]
    pub async fn load_rom_url(&self, url: String) -> Option<bool> {
        let res = http::RequestBuilder::new(&url)
            .method(http::Method::GET)
            .send()
            .await
            .ok()?;
        let bytes = res.binary().await.ok()?;

        self.load_rom_bytes(bytes);

        Some(true)
    }
}
