use gloo::net::http;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, wasm_bindgen, HtmlCanvasElement};
use winit::event_loop::EventLoopProxy;

use nes::Region;
use ui::audio::Audio;

mod app;
mod audio;
mod gfx;
mod gfx_worker;
mod gl;
mod machine_worker;
mod sync;

use app::UserEvent;

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
        let (audio, sync, samples_producer) =
            ui::audio::CpalAudio::new(sync, region.refresh_rate(), 128).ok()?;

        let sample_rate = audio.sample_rate();

        let gfx_worker = gfx_worker::GfxWorker::new(&canvas).await;
        let back_buffer = gfx_worker.back_buffer.clone();
        let mut app = app::App::new(gfx_worker, audio, sync, canvas);

        let nes_inputs = app.nes_io();

        machine_worker::MachineWorker::new(
            nes_inputs,
            back_buffer,
            samples_producer,
            region,
            sample_rate,
        )
        .await;

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
