extern crate gcc;

use std::env;

fn main() {
	gcc::compile_library("libnes_ntsc.a", &["nes_ntsc.c"]);
}
