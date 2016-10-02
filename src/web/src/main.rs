#![feature(libc)]
extern crate libc;
extern "C" {
    fn emscripten_asm_const_int(script: *const libc::c_char, ...) -> libc::c_int;
}

macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const _
    }
}

macro_rules! js {
    (($($name:ident = $expr:expr),*) {$($tt:tt)*}) => {
        unsafe { emscripten_asm_const_int(c_str!(concat!(
            $(
                "var ", stringify!($name), " = $0; ",
            )*
             stringify!($($tt)*))), $($expr),*) }
    };
}

macro_rules! js_raw {
    (($($name:ident = $expr:expr),*) {$code:expr}) => {
        unsafe { emscripten_asm_const_int(c_str!(concat!(
            $(
                "var ", stringify!($name), " = $0; ",
            )*
             $code)), $($expr),*) }
    };
}

extern crate mass_nes;

use mass_nes::nes::{UserInput, Controller, Machine, Cartridge, Region};

fn main() {
    let rom = include_bytes!("/home/nickmass/kirby.nes");
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut (rom as &[u8])).unwrap();

    let mut machine = Machine::new(region, cart, |screen| {
        let screen: Vec<u32> = screen.iter().map(|i| {
            let mut color: u32 = 0;
            color |= pal[(i*3) as usize] as u32;
            color |= (pal[((i*3) + 1) as usize] as u32) << 8;
            color |= (pal[((i*3) + 2) as usize] as u32) << 16;
            color
        }).collect();
        js_raw!{(js_screen = &*screen){r#"var screen = Module.HEAPU32.subarray(js_screen >> 2, (js_screen >> 2)+(256*240));postMessage(screen);"#}};
    }, |samples| {
    }, || {
        let mut r = Vec::new();

        let p1 = Controller {
            a: false,
            b: false,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
        };

        r.push(UserInput::PlayerOne(p1));
        r
    }, |sys, state| {});

    machine.run();
}
