#![allow(dead_code)]
#![allow(unused_variables)]

#[macro_use]
extern crate glium;
extern crate blip_buf;
extern crate clap;

use blip_buf::BlipBuf;

mod nes;
use nes::{UserInput, Controller, Machine, Cartridge, Region};

mod ui;
use ui::gfx::{Key, Renderer};
use ui::audio::{Audio, RodioAudio};
use ui::sync::FrameSync;

use clap::{App, Arg, SubCommand};

use std::sync::Mutex;
use std::cell::RefCell;
use std::rc::Rc;
use std::fs::File;

fn main() {
    let matches = App::new("Mass Nes")
        .author("Nick Massey, nickmass@nickmass.com")
        .about("Nintendo Entertainment System Emulator")
        .arg(Arg::with_name("file")
             .takes_value(true)
             .default_value("/home/nickmass/smb.nes")
             .index(1)
             .validator(|f| {
                 if ::std::path::Path::new(&f).exists() {
                     Ok(())
                 } else {
                     Err("File does not exist".to_string())
                 }
             })
             .global(true)
        )
        .arg(Arg::with_name("region")
             .short("r")
             .long("region")
             .default_value("ntsc")
             .possible_values(&["ntsc", "pal"])
             .global(true)
        )
        .subcommand(SubCommand::with_name("bench")
                    .about("Benchmark core performance")
                    .arg(Arg::with_name("frames")
                         .short("f")
                         .long("frames")
                         .default_value("0")
                         .validator(|f| {
                             let frames: Result<u32,_> = f.parse();
                             frames.map(|v| ()).map_err(|e| "Invalid frames value".to_string())
                         })
                    )
        )
        .get_matches();

    let path = matches.value_of("file").unwrap();
    let file = File::open(path.to_string()).unwrap();

    let region = if matches.value_of("region").unwrap() == "pal" {
        Region::Pal
    } else {
        Region::Ntsc
    };

    match matches.subcommand() {
        ("bench", Some(sub_matches)) => {
            let frames = sub_matches.value_of("frames").unwrap().parse().unwrap();
            bench(file, region, frames)
        },
        _ => run(file, region)
    };
}

fn run(mut file: File, region: Region) {
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut file).unwrap();

    let window = Rc::new(RefCell::new(Renderer::new(pal)));
    let mut audio = RodioAudio::new(48000);

    let sample_rate = audio.sample_rate();
    let mut delta = 0;
    let mut blip = BlipBuf::new(sample_rate / 30);
    blip.set_rates(region.frame_ticks() * region.refresh_rate(), sample_rate as f64);

    let mut frame_sync = FrameSync::new(region.refresh_rate());
    {
        let mut machine = Machine::new(region, cart, |screen| {
            window.borrow_mut().add_frame(screen);
            frame_sync.sync_frame();
        }, |samples| {
            let count = samples.len();

            for (i, v) in samples.iter().enumerate() {
                blip.add_delta(i as u32, *v as i32 - delta);
                delta = *v as i32;
            }
            blip.end_frame(count as u32);
            while blip.samples_avail() > 0 {
                let mut buf = &mut [0i16; 1024];
                let count = blip.read_samples(buf, false);
                audio.add_samples(buf[0..count].to_vec());
            }

        }, || {
            let mut r = Vec::new();
            let input = window.borrow().get_input();

            let p1 = Controller {
                a: *input.get(&Key::Z).unwrap_or(&false),
                b: *input.get(&Key::X).unwrap_or(&false),
                select: *input.get(&Key::RShift).unwrap_or(&false),
                start: *input.get(&Key::Return).unwrap_or(&false),
                up: *input.get(&Key::Up).unwrap_or(&false),
                down: *input.get(&Key::Down).unwrap_or(&false),
                left: *input.get(&Key::Left).unwrap_or(&false),
                right: *input.get(&Key::Right).unwrap_or(&false),
            };

            if *input.get(&Key::Delete).unwrap_or(&false) {
                r.push(UserInput::Power);
            }

            if *input.get(&Key::Back).unwrap_or(&false) {
                r.push(UserInput::Reset);
            }

            if window.borrow().is_closed() {
                r.push(UserInput::Close);
            }

            r.push(UserInput::PlayerOne(p1));
            r

        }, |sys, state| {});

        machine.run();
    }

    audio.close();
    if let Ok(window) = Rc::try_unwrap(window) {
        window.into_inner().close();
    }
}

fn bench(mut file: File, region: Region, frames: u32) {
    let cart = Cartridge::load(&mut file).unwrap();
    let closed = Mutex::new(false);
    let mut machine = Machine::new(region, cart,
                                  |screen| {},
                                  |samples| {},
                                  || {
                                      let mut r = Vec::new();

                                      let closed = closed.lock().unwrap();
                                      if *closed {
                                          r.push(UserInput::Close);
                                      }

                                      r
                                  },
                                  |system, state| {
                                      let mut closed = closed.lock().unwrap();
                                      let nes_frame = system.debug.frame(state);
                                      *closed = frames != 0 && nes_frame > frames;
                                  });

    machine.run();
}

fn generate_pal() {
    let mut new_c = [0; 0x40 * 8 * 3];
    let c = Region::Ntsc.default_palette();
    let emp = 0.1;
    let demp = 0.25;
    for i in 0..8 {
        let mut blue = 1.0;
        let mut green = 1.0;
        let mut red = 1.0;

        if i & 0x01 != 0 {
            red += emp;
            green -= demp;
            blue -= demp;
        }

        if i & 0x02 != 0 {
            green += emp;
            red -= demp;
            blue -= demp;
        }

        if i & 0x04 != 0 {
            blue += emp;
            red -= demp;
            green -= demp;
        }

        let red = if red < 0.0 { 0.0 } else { red };
        let green = if green < 0.0 { 0.0 } else { green };
        let blue = if blue < 0.0 { 0.0 } else { blue };
        for j in 0..0x40 {
            let x = j*3;
            let final_red = (c[x] as f64 * red).round();
            let final_red = if final_red > 255.0 { 0xff } else { final_red as u8 };
            let final_green = (c[x + 1] as f64 * green).round();
            let final_green = if final_green > 255.0 { 0xff } else { final_green as u8 };
            let final_blue = (c[x + 2] as f64 * blue).round();
            let final_blue = if final_blue > 255.0 { 0xff } else { final_blue as u8 };

            let index = (i * 192) + x;
            new_c[index as usize] = final_red;
            new_c[index as usize + 1] = final_green;
            new_c[index as usize + 2] = final_blue;
        }
    }
    //use std::io::Write;
    //let mut f = std::fs::File::create("emp_pal.pal").unwrap(); 
    //f.write_all(&new_c);
}
