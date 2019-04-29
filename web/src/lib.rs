#![recursion_limit = "1024"]
#![feature(custom_attribute)]

#[macro_use]
extern crate stdweb;

use stdweb::UnsafeTypedArray;

use nes::{Cartridge, Controller, Machine, Region, UserInput};
use std::cell::RefCell;

thread_local!(static MACHINE: RefCell<Option<Machine>> = RefCell::new(None));

#[js_export]
pub fn main() {
    js! {
        let listeners = [];
        Module.exports.addEventListener = (event, cb) => {
            listeners.push({event: event, cb: cb});
        };

        Module.exports.dispatchEvent = (event, data) => {
            listeners.filter(e => e.event === event).forEach(e => e.cb(data));
        };
    }
}

#[js_export]
pub fn load_rom(rom: Vec<u8>) {
    let region = Region::Ntsc;
    let cart = Cartridge::load(&mut rom.as_slice()).unwrap();

    MACHINE.with(|m| {
        let mut m = m.borrow_mut();
        *m = Some(Machine::new(region, cart));
    });
}

#[js_export]
pub fn run_frame(input: Vec<String>) {
    let input = input.iter().fold(Controller::new(), |mut c, i| {
        match i.as_str() {
            "Up" => c.up = true,
            "Down" => c.down = true,
            "Left" => c.left = true,
            "Right" => c.right = true,
            "A" => c.a = true,
            "B" => c.b = true,
            "Select" => c.select = true,
            "Start" => c.start = true,
            _ => (),
        }
        c
    });

    MACHINE.with(|m| {
        let mut m = m.borrow_mut();

        if let Some(m) = (*m).as_mut() {
            let pal = Region::Ntsc.default_palette();
            m.set_input(vec![UserInput::PlayerOne(input)]);
            m.run();
            let screen: Vec<u8> = m.get_screen().iter().fold(Vec::new(), |mut screen, i| {
                let red = pal[(i * 3) as usize];
                let green = pal[((i * 3) + 1) as usize];
                let blue = pal[((i * 3) + 2) as usize];
                screen.push(red);
                screen.push(green);
                screen.push(blue);
                screen.push(255);

                screen
            });
            let screen_slice = unsafe { UnsafeTypedArray::new(&*screen) };
            js! {
                let screenSlice = @{screen_slice};
                Module.exports.dispatchEvent("screen", screenSlice);
            }
            let audio: Vec<_> = {
                let samples = m.get_audio();
                let rate = (samples.len() as f32 / (48000.0 / 60.0)) as usize;
                samples
                    .chunks(rate)
                    .map(|c| {
                        c.iter().map(|s| *s as f32).sum::<f32>()
                            / (c.len() as f32 * i16::max_value() as f32)
                    })
                    .collect()
            };

            let audio_slice = unsafe { UnsafeTypedArray::new(&*audio) };
            js! {
                let audioSlice = @{audio_slice};
                Module.exports.dispatchEvent("audio", audioSlice);
            }
        }
    });
}
