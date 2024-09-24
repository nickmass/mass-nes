use crossbeam::sync::{Parker, Unparker};
use wasm_bindgen::prelude::*;
use web_sys::{js_sys::Array, wasm_bindgen};
use winit::event_loop::EventLoopProxy;

use crate::{app::UserEvent, worker::WorkerSpawn};

pub trait FrameSync {
    fn sync_frame(&mut self);
}

pub struct CpalSync {
    parker: Parker,
}

impl CpalSync {
    pub fn new() -> Self {
        Self {
            parker: Parker::new(),
        }
    }
}

impl ui::audio::Parker for CpalSync {
    type Unparker = CpalUnparker;

    fn unparker(&self) -> Self::Unparker {
        CpalUnparker(self.parker.unparker().clone())
    }
}

pub struct CpalUnparker(Unparker);

impl ui::audio::Unparker for CpalUnparker {
    fn unpark(&mut self) {
        self.0.unpark()
    }
}

impl FrameSync for CpalSync {
    fn sync_frame(&mut self) {
        self.parker.park()
    }
}

pub struct SyncSpawner<T> {
    sync: T,
    proxy: EventLoopProxy<UserEvent>,
}

impl<T> SyncSpawner<T> {
    pub fn new(sync: T, proxy: EventLoopProxy<UserEvent>) -> Self {
        Self { sync, proxy }
    }
}

#[wasm_bindgen]
pub async fn sync_worker(ptr: u32, transferables: Array) {
    crate::worker::worker::<SyncSpawner<CpalSync>>(ptr, transferables).await
}

impl<T: FrameSync + Send + 'static> WorkerSpawn for SyncSpawner<T> {
    const KIND: &'static str = "sync";

    async fn run(mut self, _transferables: Array) {
        loop {
            self.sync.sync_frame();
            let _ = self.proxy.send_event(UserEvent::Sync);
        }
    }
}
