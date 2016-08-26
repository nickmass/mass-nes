#[macro_use]
extern crate glium;

mod nes;
use nes::{Controller, Machine, Cartridge, Region};

mod ui;
use ui::gfx::GliumRenderer;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut file = ::std::fs::File::open(std::env::args().nth(1).unwrap_or("/home/nickmass/smb.nes".to_string())).unwrap();
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut file).unwrap();
    
    let renderer = Rc::new(RefCell::new(GliumRenderer::new(pal)));
    let mut machine = Machine::new(region, cart, |screen| {
        renderer.borrow_mut().render(screen);
        println!("Rendered");
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
    });

    machine.run();
}
