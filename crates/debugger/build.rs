fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() == Some("wasm32") {
        create_audio_worklet();
    }
}

pub fn create_audio_worklet() {
    let worklet_source = ui::audio::worket_module_source("./pkg/debugger.js");
    std::fs::write("static/worklet.js", worklet_source).unwrap();
}
