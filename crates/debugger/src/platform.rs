pub use platform::*;

#[cfg(not(target_arch = "wasm32"))]
use desktop as platform;
#[cfg(target_arch = "wasm32")]
use web as platform;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    #[derive(Copy, Debug, Clone)]
    pub struct Timestamp(std::time::Instant);

    impl Timestamp {
        pub fn now() -> Self {
            Self(std::time::Instant::now())
        }

        pub fn duration_since(&self, Timestamp(start): Timestamp) -> std::time::Duration {
            self.0.duration_since(start)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;
    use web_sys::{js_sys, WorkerGlobalScope};
    #[derive(Copy, Debug, Clone)]
    pub struct Timestamp(f64);

    impl Timestamp {
        pub fn now() -> Self {
            let perf = if let Some(window) = web_sys::window() {
                window.performance().unwrap_throw()
            } else if let Some(worker) = js_sys::global().dyn_into::<WorkerGlobalScope>().ok() {
                worker.performance().unwrap_throw()
            } else {
                panic!("must have window")
            };
            let now = perf.now();
            Self(now)
        }

        pub fn duration_since(&self, Timestamp(start): Timestamp) -> std::time::Duration {
            let ms = self.0 - start;
            std::time::Duration::from_secs_f64(ms / 1000.0)
        }
    }
}
