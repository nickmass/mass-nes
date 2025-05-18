#[cfg(target_arch = "wasm32")]
use crate as debugger;
use tracing::Level;
use tracing_subscriber::{Layer, filter, layer::SubscriberExt};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use debugger::{DebuggerApp, EguiMessageLayer, MessageStore};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use ui::audio::Audio;
    let message_store = MessageStore::new(10_000);
    init_tracing(message_store.clone());

    let options = eframe::NativeOptions {
        vsync: false,
        multisampling: 2,
        ..Default::default()
    };

    #[cfg(not(feature = "jack"))]
    let (mut audio, samples_tx) = ui::audio::CpalAudio::new().unwrap();
    #[cfg(feature = "jack")]
    let (mut audio, samples_tx) = ui::audio::JackAudio::new().unwrap();

    audio.pause();

    eframe::run_native(
        debugger::APP_NAME,
        options,
        Box::new(|cc| {
            Ok(Box::new(DebuggerApp::new(
                cc,
                message_store,
                audio,
                samples_tx,
            )?))
        }),
    )
    .unwrap();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn main() {
    let message_store = MessageStore::new(10_000);
    init_tracing(message_store.clone());
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");

        let canvas = document
            .get_element_by_id("render_canvas")
            .expect("failed to find render_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("render_canvas was not a HtmlCanvasElement");

        let (audio, samples_tx) = ui::audio::BrowserAudio::new("worklet.js")
            .await
            .expect("failed to init audio");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(Box::new(DebuggerApp::new(
                        cc,
                        message_store,
                        audio,
                        samples_tx,
                    )?))
                }),
            )
            .await;

        match start_result {
            Err(e) => tracing::error!("app failed: {:?}", e),
            _ => (),
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn init_tracing(message_store: MessageStore) {
    let log = tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::DEBUG);
    let tracy =
        tracing_tracy::TracyLayer::default().with_filter(filter::Targets::new().with_targets([
            ("debugger", Level::TRACE),
            ("nes", Level::TRACE),
            ("ui", Level::TRACE),
        ]));

    let messages =
        EguiMessageLayer::new(message_store).with_filter(filter::Targets::new().with_targets([
            ("debugger", Level::TRACE),
            ("nes", Level::TRACE),
            ("ui", Level::TRACE),
        ]));

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracy)
            .with(log)
            .with(messages),
    )
    .expect("init tracing");
}

#[cfg(target_arch = "wasm32")]
fn init_tracing(message_store: MessageStore) {
    let messages =
        EguiMessageLayer::new(message_store).with_filter(filter::Targets::new().with_targets([
            ("debugger", Level::TRACE),
            ("nes", Level::TRACE),
            ("ui", Level::TRACE),
        ]));

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(wasm_tracing::WasmLayer::default())
            .with(messages),
    )
    .expect("init tracing");
}
