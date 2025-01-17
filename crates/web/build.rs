fn main() {
    let worklet_source = ui::audio::worket_module_source("./pkg/web.js");
    std::fs::write("static/worklet.js", worklet_source).unwrap();
}
