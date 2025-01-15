use gloo::net::http;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, wasm_bindgen, HtmlCanvasElement};
use winit::event_loop::EventLoopProxy;

use nes::Region;
use ui::{audio::Audio, sync::EmuSync};

mod app;
mod gfx;
mod gl;
mod offscreen_gfx;
mod runner;
mod sync;

use app::UserEvent;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    wasm_tracing::set_as_global_default();
}

#[wasm_bindgen]
pub struct Emulator {
    proxy: EventLoopProxy<UserEvent>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub async fn new(region: String, canvas: HtmlCanvasElement) -> Result<Emulator, JsError> {
        let region = region.to_lowercase();
        let region = match region.as_str() {
            "pal" => Region::Pal,
            "ntsc" | _ => Region::Ntsc,
        };

        let (audio, audio_sync, samples_producer) =
            ui::audio::CpalAudio::new(region.refresh_rate())?;

        let sample_rate = audio.sample_rate();

        let gfx_worker = offscreen_gfx::GfxWorker::new(&canvas).await?;
        let back_buffer = gfx_worker.back_buffer.clone();
        let mut app = app::App::new(gfx_worker, audio, canvas)?;

        let emu_sync = EmuSync::new();
        let sync_spawner = sync::SyncSpawner::new(audio_sync, emu_sync.clone(), app.proxy());
        web_worker::spawn_worker(sync_spawner).await?;

        let nes_inputs = app.nes_io();

        let machine_spawner = runner::MachineSpawner::new(
            region,
            sample_rate,
            back_buffer,
            samples_producer,
            emu_sync,
            nes_inputs,
        );
        web_worker::spawn_worker(machine_spawner).await?;

        let proxy = app.proxy();
        app.run();

        Ok(Self { proxy })
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

    #[wasm_bindgen]
    pub fn run_bench(&self, rom: js_sys::ArrayBuffer, frames: u32) -> Option<bool> {
        use nes::{Cartridge, Machine};
        use std::io::Cursor;
        let rom = js_sys::Uint8Array::new(&rom);
        let rom = rom.to_vec();
        let mut cursor = Cursor::new(rom);
        let cart = Cartridge::load(&mut cursor).unwrap();
        let mut machine = Machine::new(Region::Ntsc, cart);
        let window = web_sys::window()?;
        let performance = window.performance()?;

        let start = performance.now();
        for _ in 0..frames {
            machine.run();
            let _ = machine.get_audio();
        }
        let end = performance.now();
        let elapsed = std::time::Duration::from_secs_f64((end - start) / 1000.0);
        let fps = frames as f64 / elapsed.as_secs_f64();

        tracing::info!(
            "Benchmark {} frames in {}.{:03} seconds, {:.3}fps",
            frames,
            elapsed.as_secs(),
            elapsed.subsec_millis(),
            fps
        );

        Some(true)
    }
}
