extern crate cc;

fn main() {
    cc::Build::new().file("nes_ntsc.c").compile("libnes_ntsc.a");
}
