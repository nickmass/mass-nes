#![allow(dead_code)]
#![allow(unused_variables)]

#[macro_use]
extern crate glium;
extern crate blip_buf;

use blip_buf::BlipBuf;

const CLOCK_RATE: f64 = 29780.5 * 60.0;

mod nes;
use nes::{Controller, Machine, Cartridge, Region};

mod ui;
use ui::gfx::GliumRenderer;
use ui::audio::Audio;

use std::cell::RefCell;
use std::rc::Rc;
use std::fs;
use std::env;

fn main() {
    let mut file = fs::File::open(env::args().nth(1).unwrap_or("/home/nickmass/smb.nes".to_string())).unwrap();
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut file).unwrap();
    
    let renderer = Rc::new(RefCell::new(GliumRenderer::new(pal)));
    let mut audio = Audio::new();
    let sample_rate = audio.sample_rate();
   
    let mut delta = 0;
    let mut blip = BlipBuf::new(sample_rate / 30);
    //TODO - Should be region.refresh_rate instead of 60.0.
    //Currently we are syncing to computer vsync instead of console framerate
    blip.set_rates(region.frame_ticks() * 60.0, sample_rate as f64);

    let mut machine = Machine::new(region, cart, |screen| {
        renderer.borrow_mut().render(screen);
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
        renderer.borrow().is_closed()
    }, || {
        let input = renderer.borrow().get_input();
        Controller {
            a: input[0],
            b: input[1],
            select: input[2],
            start: input[3],
            up: input[4],
            down: input[5],
            left: input[6],
            right: input[7],
        }
    }, |sys, state| {});

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
