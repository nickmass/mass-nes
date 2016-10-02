extern crate mass_nes;

use mass_nes::nes::{UserInput, Controller, Machine, Cartridge, Region};

fn main() {
    let rom = include_bytes!("/home/nickmass/kirby.nes");
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut (rom as &[u8])).unwrap();

    let mut machine = Machine::new(region, cart, |screen| {
        let mut string = screen.iter().map(|i| {
            let mut color: u32 = 0;
            color |= pal[(i*3) as usize] as u32;
            color |= (pal[((i*3) + 1) as usize] as u32) << 8;
            color |= (pal[((i*3) + 2) as usize] as u32) << 16;
            color
        }).fold(String::new(), |mut acc, c| {
            if acc.len() == 0 {
                acc.push_str(&format!("[{}", c));
            } else {
                acc.push_str(&format!(",{}", c));
            }
            acc
        });
        string.push(']');
        println!("{}", string);
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
