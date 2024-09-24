use js_sys::Array;
use wasm_bindgen::{closure::Closure, prelude::*, JsValue};
use web_sys::{
    js_sys::{self, Object},
    wasm_bindgen, DedicatedWorkerGlobalScope, MessageEvent, Worker, WorkerOptions,
};

pub trait WorkerSpawn: Send + Sized + 'static {
    const KIND: &'static str;

    async fn run(self, transferables: Array);
}

pub async fn spawn_worker<T: WorkerSpawn>(spawner: T) -> Result<(), SpawnError> {
    spawn_worker_with_transfer(spawner, None).await
}

pub async fn spawn_worker_with_transfer<T: WorkerSpawn>(
    spawner: T,
    transferables: impl IntoIterator<Item = JsValue>,
) -> Result<(), SpawnError> {
    let spawner = Box::new(spawner);
    let spawner = Box::into_raw(spawner);

    let arr = Array::new();

    for val in transferables {
        arr.push(&val.into());
    }

    if let Err(err) = try_spawn_worker(spawner, arr).await {
        let _ = unsafe { Box::from_raw(spawner) };
        Err(err)
    } else {
        Ok(())
    }
}

pub fn unpack_transferable<T: JsCast>(transferables: &Array) -> Option<T> {
    for val in transferables.iter() {
        if let Ok(val) = val.dyn_into::<T>() {
            return Some(val);
        }
    }

    None
}

async fn try_spawn_worker<T: WorkerSpawn>(
    spawner: *mut T,
    transferables: Array,
) -> Result<(), SpawnError> {
    let opts = WorkerOptions::new();
    opts.set_type(web_sys::WorkerType::Module);
    let worker = Worker::new_with_options("worker.js", &opts)?;

    let (init_tx, init_rx) = futures::channel::oneshot::channel();
    let on_message = Closure::once_into_js(move |msg: MessageEvent| {
        if msg.data().is_truthy() {
            init_tx.send(()).unwrap_throw();
        }
    });
    worker.set_onmessage(Some(&on_message.as_ref().unchecked_ref()));

    let init: JsValue = WorkerInit::<T>::new(spawner as u32, transferables.clone()).into();
    worker.post_message_with_transfer(&init, transferables.as_ref())?;
    init_rx.await?;

    Ok(())
}

pub async fn worker<T: WorkerSpawn>(ptr: u32, transferables: Array) {
    let worker = unsafe { Box::from_raw(ptr as *mut T) };
    let global: DedicatedWorkerGlobalScope = js_sys::global().dyn_into().unwrap_throw();
    global.post_message(&JsValue::TRUE).unwrap_throw();

    worker.run(transferables).await
}

#[derive(Debug, Clone)]
pub enum SpawnError {
    Js(JsValue),
    Canceled,
}

impl std::error::Error for SpawnError {}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnError::Js(val) => write!(
                f,
                "{}",
                val.as_string().unwrap_or(format!("js error: {val:?}")),
            ),
            SpawnError::Canceled => write!(f, "remote worker dropped before completing init"),
        }
    }
}

impl From<JsValue> for SpawnError {
    fn from(value: JsValue) -> Self {
        Self::Js(value)
    }
}

impl From<futures::channel::oneshot::Canceled> for SpawnError {
    fn from(_value: futures::channel::oneshot::Canceled) -> Self {
        SpawnError::Canceled
    }
}

struct WorkerInit<T> {
    memory: JsValue,
    module: JsValue,
    ptr: u32,
    transferables: Array,
    _marker: std::marker::PhantomData<T>,
}

impl<T> WorkerInit<T> {
    fn new(ptr: u32, transferables: Array) -> Self {
        Self {
            memory: wasm_bindgen::memory(),
            module: wasm_bindgen::module(),
            ptr,
            transferables,
            _marker: Default::default(),
        }
    }
}

impl<T: WorkerSpawn> Into<JsValue> for WorkerInit<T> {
    fn into(self) -> JsValue {
        let obj = Object::new();
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("worker_type"),
            &JsValue::from_str(T::KIND),
        );
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("memory"), &self.memory);
        let _ = js_sys::Reflect::set(obj.as_ref(), &JsValue::from_str("module"), &self.module);
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("ptr"),
            &JsValue::from_f64(self.ptr as f64),
        );
        let _ = js_sys::Reflect::set(
            obj.as_ref(),
            &JsValue::from_str("transferables"),
            &self.transferables,
        );

        obj.into()
    }
}
