use gloo::net::http;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, wasm_bindgen, HtmlCanvasElement};
use winit::event_loop::EventLoopProxy;

use nes::Region;
use ui::audio::Audio;

mod app;
mod gfx;
mod gl;
mod offscreen_gfx;
mod runner;

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

        #[cfg(target_arch = "wasm32")]
        let (audio, samples_tx) =
            ui::audio::BrowserAudio::new("worklet.js", region.refresh_rate()).await?;
        #[cfg(not(target_arch = "wasm32"))]
        let (audio, samples_tx) = ui::audio::Null::new();

        let sample_rate = audio.sample_rate();

        let gfx_worker = offscreen_gfx::GfxWorker::new(&canvas).await?;
        let back_buffer = gfx_worker.back_buffer.clone();
        let mut app = app::App::new(gfx_worker, audio, canvas)?;

        let nes_inputs = app.nes_io();

        let machine_spawner =
            runner::MachineSpawner::new(region, sample_rate, back_buffer, samples_tx, nes_inputs);
        web_worker::spawn_worker(machine_spawner).await?;

        let proxy = app.proxy();
        app.run();

        Ok(Self { proxy })
    }

    fn load_rom_bytes(&self, bytes: Vec<u8>) {
        let _ = self.proxy.send_event(UserEvent::Load(bytes));
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
        let cart = Cartridge::load(&mut cursor, None, None, "bench.nes").unwrap();
        let mut machine = Machine::new(Region::Ntsc, cart);
        let window = web_sys::window()?;
        let performance = window.performance()?;

        let start = performance.now();
        for _ in 0..frames {
            machine.run();
            let _ = machine.get_samples();
        }
        let end = performance.now();
        let elapsed = std::time::Duration::from_secs_f64((end - start) / 1000.0);
        let fps = frames as f64 / elapsed.as_secs_f64();

        tracing::info!(
            "benchmark {} frames in {}.{:03} seconds, {:.3}fps",
            frames,
            elapsed.as_secs(),
            elapsed.subsec_millis(),
            fps
        );

        Some(true)
    }
}
