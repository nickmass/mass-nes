use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    SinkExt, StreamExt,
};
use web_sys::{
    js_sys::{self, Array, Object},
    wasm_bindgen::{self, closure::Closure, prelude::*, JsCast, JsValue},
    DedicatedWorkerGlobalScope, HtmlCanvasElement, OffscreenCanvas, Worker, WorkerOptions,
};

#[wasm_bindgen]
pub async fn worker_gfx(offscreen_canvas: OffscreenCanvas, channel: u32) {
    let mut channel = unsafe { GfxWorkerChannel::from_raw(channel) };
    let global: DedicatedWorkerGlobalScope = js_sys::global().dyn_into().unwrap();
    global.post_message(&JsValue::TRUE).unwrap();
    channel.tx.send(GfxResponse::Init).await.unwrap();

    let setup = ui::filters::NesNtscSetup::composite();
    let filter = ui::filters::NtscFilter::new(&setup);
    //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());
    let gfx = crate::gfx::Gfx::new(offscreen_canvas, filter, *channel);
    gfx.run().await;
}

pub struct GfxWorker {
    pub tx: Sender<GfxRequest>,
    pub rx: Receiver<GfxResponse>,
}

impl GfxWorker {
    pub async fn new(canvas: &HtmlCanvasElement) -> Self {
        let off_screen = canvas.transfer_control_to_offscreen().unwrap();
        let opts = WorkerOptions::new();
        opts.set_type(web_sys::WorkerType::Module);
        let worker = Worker::new_with_options("worker_gfx.js", &opts).unwrap();

        let (my_tx, their_rx) = channel(100);
        let (their_tx, mut my_rx) = channel(100);

        let (init_tx, init_rx) = futures::channel::oneshot::channel();
        let on_message = Closure::once_into_js(move || {
            init_tx.send(()).unwrap_throw();
        });
        worker.set_onmessage(Some(&on_message.as_ref().unchecked_ref()));

        let channel = GfxWorkerChannel {
            tx: their_tx,
            rx: their_rx,
        };

        let init: JsValue = GfxWorkerInit::new(off_screen.clone(), channel).into();
        let arr = Array::new();
        arr.push(&off_screen);
        worker.post_message_with_transfer(&init, &arr).unwrap();
        init_rx.await.unwrap();
        let Some(GfxResponse::Init) = my_rx.next().await else {
            panic!("GfxWorker init failed");
        };

        GfxWorker {
            tx: my_tx,
            rx: my_rx,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum GfxResponse {
    Init,
}

#[derive(Debug, Clone)]
pub enum GfxRequest {
    Frame(Vec<u16>),
    Redraw,
    Resize(u32, u32),
}

pub struct GfxWorkerChannel {
    pub tx: Sender<GfxResponse>,
    pub rx: Receiver<GfxRequest>,
}

impl GfxWorkerChannel {
    unsafe fn from_raw(raw: u32) -> Box<Self> {
        Box::from_raw(raw as *mut _)
    }
}

struct GfxWorkerInit {
    memory: JsValue,
    module: JsValue,
    offscreen_canvas: OffscreenCanvas,
    channel: u32,
}

impl GfxWorkerInit {
    fn new(offscreen_canvas: OffscreenCanvas, channel: GfxWorkerChannel) -> Self {
        let channel = Box::new(channel);
        let channel = Box::into_raw(channel) as u32;
        Self {
            memory: wasm_bindgen::memory(),
            module: wasm_bindgen::module(),
            offscreen_canvas,
            channel,
        }
    }
}

impl Into<JsValue> for GfxWorkerInit {
    fn into(self) -> JsValue {
        let obj = Object::new();
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("memory"), &self.memory);
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("module"), &self.module);
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("offscreen_canvas"),
            &self.offscreen_canvas,
        );
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("channel"),
            &JsValue::from_f64(self.channel as f64),
        );

        obj.into()
    }
}
