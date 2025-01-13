#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use debugger::{DebuggerApp, EguiMessageLayer, MessageStore};

    let message_store = MessageStore::new(10_000);
    init_tracing(message_store.clone());

    let options = eframe::NativeOptions {
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "Mass Emu",
        options,
        Box::new(|cc| Ok(Box::new(DebuggerApp::new(cc, message_store)?))),
    )
    .unwrap();

    fn init_tracing(message_store: MessageStore) {
        use tracing::Level;
        use tracing_subscriber::{filter, layer::SubscriberExt, Layer};

        let tracy = tracing_tracy::TracyLayer::default().with_filter(
            filter::Targets::new().with_targets([
                ("debugger", Level::TRACE),
                ("nes", Level::TRACE),
                ("ui", Level::TRACE),
            ]),
        );
        let log = tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::DEBUG);
        let messages = EguiMessageLayer::new(message_store).with_filter(
            filter::Targets::new().with_targets([
                ("debugger", Level::TRACE),
                ("nes", Level::TRACE),
                ("ui", Level::TRACE),
            ]),
        );

        tracing::subscriber::set_global_default(
            tracing_subscriber::registry()
                .with(tracy)
                .with(log)
                .with(messages),
        )
        .expect("init tracing");
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
fn main() {
    web_sys::console::log_1(&JsValue::from_str("hi"));

    use crate::{DebuggerApp, EguiMessageLayer, MessageStore};

    let message_store = MessageStore::new(10_000);
    web_sys::console::log_1(&JsValue::from_str("two"));
    init_tracing(message_store.clone());
    tracing::error!("tracing init");
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("render_canvas")
            .expect("Failed to find render_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("render_canvas was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(DebuggerApp::new(cc, message_store)?))),
            )
            .await;

        match start_result {
            Err(e) => tracing::error!("app failed: {:?}", e),
            _ => (),
        }
    });

    fn init_tracing(message_store: MessageStore) {
        use tracing::Level;
        use tracing_subscriber::{filter, layer::SubscriberExt, Layer};

        web_sys::console::log_1(&JsValue::from_str("1"));
        let log = wasm_tracing::WasmLayer::default();
        let messages = EguiMessageLayer::new(message_store).with_filter(
            filter::Targets::new().with_targets([
                ("debugger", Level::TRACE),
                ("nes", Level::TRACE),
                ("ui", Level::TRACE),
            ]),
        );
        web_sys::console::log_1(&JsValue::from_str("2"));

        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(log).with(messages),
        )
        .expect("init tracing");
        web_sys::console::log_1(&JsValue::from_str("3"));
    }
}
