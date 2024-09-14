fn main() {
    cc::Build::new()
        .file("nes_ntsc/nes_ntsc.c")
        .compile("libnes_ntsc.a");
}
