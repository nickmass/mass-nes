use std::os::raw::{c_char, c_int};

extern {
    fn emscripten_asm_const_int(script: *const c_char, ...) -> c_int;
}

macro_rules! c_str {
    ($s:expr) => {
        {
            concat!($s, "\0").as_ptr() as *const _
        }
    }
}

macro_rules! js {
    (($($expr:expr),*) {$code:expr}) => {
        unsafe {
            emscripten_asm_const_int(c_str!($code), $($expr),*)
        }
    };
}

extern crate nes;

use nes::{UserInput, Controller, Machine, Cartridge, Region};

fn main() {
    let rom = include_bytes!("/home/nickmass/kirby.nes");
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut (rom as &[u8])).unwrap();

    let mut machine = Machine::new(region, cart, |screen| {
        let screen: Vec<u8> = screen.iter().fold( Vec::new(), |mut screen, i| {
            let red = pal[(i*3) as usize];
            let green = pal[((i*3) + 1) as usize];
            let blue = pal[((i*3) + 2) as usize];
            screen.push(red);
            screen.push(green);
            screen.push(blue);
            screen.push(255);

            screen
        });
        js!{(&*screen){ r#"var screen = Module.HEAPU8.subarray($0, $0+(256*240*4));postMessage({screen: screen});"#}};
    }, |samples| {
        let rate = (samples.len() as f32 / (48000.0 / 60.0)) as usize;
        let audio: Vec<_> = samples
            .chunks(rate)
            .map(|c| c.iter().map(|s| *s as f32).sum::<f32>() / (c.len() as f32 * i16::max_value() as f32))
            .collect();
        js!{(&*audio, audio.len()){ r#"var audio = Module.HEAPF32.subarray($0 >> 2, ($0 >> 2) + $1);postMessage({audio: audio});"#}};
    }, || {
        vec![UserInput::PlayerOne(Controller::new())]
    }, |_sys, _state| {});

    machine.run();
}
