use futures::channel::mpsc::{channel, Receiver, Sender};
use web_sys::{
    js_sys::Array,
    wasm_bindgen::{self, prelude::*},
    HtmlCanvasElement,
};

use crate::gfx::{GfxBackBuffer, GfxRequest};
use crate::worker::{self, WorkerSpawn};

pub struct OffscreenGfxSpawner {
    pub rx: Receiver<GfxRequest>,
    pub back_buffer: GfxBackBuffer,
}

#[wasm_bindgen]
pub async fn gfx_worker(ptr: u32, transferables: Array) {
    crate::worker::worker::<OffscreenGfxSpawner>(ptr, transferables).await
}

impl WorkerSpawn for OffscreenGfxSpawner {
    const KIND: &'static str = "gfx";

    async fn run(self, transferables: Array) {
        let canvas = worker::unpack_transferable(&transferables).unwrap_throw();
        let setup = ui::filters::NesNtscSetup::composite();
        let filter = ui::filters::NtscFilter::new(&setup);
        //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());

        let gfx = crate::gfx::Gfx::new(filter, self, canvas);
        gfx.run().await;
    }
}

pub struct GfxWorker {
    pub tx: Sender<GfxRequest>,
    pub back_buffer: GfxBackBuffer,
}

impl GfxWorker {
    pub async fn new(canvas: &HtmlCanvasElement) -> Result<Self, worker::SpawnError> {
        let off_screen = canvas.transfer_control_to_offscreen()?;
        let (my_tx, their_rx) = channel(100);
        let back_buffer = GfxBackBuffer::new(my_tx.clone());

        let channel = OffscreenGfxSpawner {
            rx: their_rx,
            back_buffer: back_buffer.clone(),
        };

        crate::worker::spawn_worker_with_transfer(channel, Some(off_screen.into())).await?;

        Ok(GfxWorker {
            tx: my_tx.clone(),
            back_buffer,
        })
    }
}