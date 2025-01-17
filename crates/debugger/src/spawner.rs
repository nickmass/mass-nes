#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys::Array;
#[cfg(target_arch = "wasm32")]
use web_worker::worker;

use crate::app::EmulatorCommands;
use crate::debug_state::DebugSwapState;
use crate::gfx::GfxBackBuffer;
use ui::audio::SamplesSender;

pub trait Spawn: Sized + Send + 'static {
    const NAME: &'static str;
    fn run(self);

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(self) {
        let _ = std::thread::Builder::new()
            .name(Self::NAME.into())
            .spawn(move || self.run());
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn(self) {
        let spawn = async move {
            let res = web_worker::spawn_worker(web::WasmSpawn(self)).await;
            match res {
                Err(e) => tracing::error!("unable to spawn '{}': {:?}", Self::NAME, e),
                _ => (),
            }
        };

        wasm_bindgen_futures::spawn_local(spawn);
    }
}

pub struct MachineSpawner {
    pub emu_commands: EmulatorCommands,
    pub back_buffer: GfxBackBuffer,
    pub samples_tx: SamplesSender,
    pub sample_rate: u32,
    pub debug: DebugSwapState,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn machine_worker(ptr: u32, transferables: Array) {
    worker::<web::WasmSpawn<MachineSpawner>>(ptr, transferables).await
}

impl Spawn for MachineSpawner {
    const NAME: &'static str = stringify!(machine_worker);

    fn run(self) {
        let MachineSpawner {
            emu_commands,
            back_buffer,
            samples_tx,
            sample_rate,
            debug,
        } = self;

        let runner =
            crate::runner::Runner::new(emu_commands, back_buffer, samples_tx, sample_rate, debug);
        runner.run()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct GamepadSpawner {
    pub gamepad: ui::gamepad::GilrsInput<crate::app::AppEventsProxy>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Spawn for GamepadSpawner {
    const NAME: &'static str = stringify!(gamepad_worker);

    fn run(mut self) {
        loop {
            self.gamepad.poll_blocking();
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use web_sys::js_sys::Array;
    use web_worker::WorkerSpawn;

    impl<T: super::Spawn> WorkerSpawn for WasmSpawn<T> {
        const ENTRY_POINT: &'static str = T::NAME;

        async fn run(self, _transferables: Array) {
            self.0.run()
        }
    }

    pub struct WasmSpawn<T>(pub T);
}
