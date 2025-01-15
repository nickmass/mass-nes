use ui::sync::FrameSync;
use ui::{audio::SamplesSync, sync::EmuSync};
use wasm_bindgen::prelude::*;
use web_sys::{js_sys::Array, wasm_bindgen};
use web_worker::WorkerSpawn;
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;

pub struct SyncSpawner {
    audio_sync: SamplesSync,
    emu_sync: EmuSync,
    proxy: EventLoopProxy<UserEvent>,
}

impl SyncSpawner {
    pub fn new(
        audio_sync: SamplesSync,
        emu_sync: EmuSync,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        Self {
            audio_sync,
            emu_sync,
            proxy,
        }
    }
}

#[wasm_bindgen]
pub async fn sync_worker(ptr: u32, transferables: Array) {
    web_worker::worker::<SyncSpawner>(ptr, transferables).await
}

impl WorkerSpawn for SyncSpawner {
    const ENTRY_POINT: &'static str = stringify!(sync_worker);

    async fn run(mut self, _transferables: Array) {
        loop {
            self.audio_sync.sync_frame();
            if !self.emu_sync.request_run() {
                let _ = self.proxy.send_event(UserEvent::Sync);
            }
        }
    }
}
