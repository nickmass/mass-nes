#[macro_use]
extern crate glium;

mod nes;
use nes::{Machine, Cartridge, Region};

mod ui;
use ui::gfx::GliumRenderer;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut file = ::std::fs::File::open("/home/nickmass/balloon.nes").unwrap();
    let region = Region::Ntsc;
    let pal = region.default_palette();
    let cart = Cartridge::load(&mut file).unwrap();
    
    let renderer = Rc::new(RefCell::new(GliumRenderer::new(pal)));
    let mut machine = Machine::new(region, cart, |screen| {
        renderer.borrow_mut().render(screen);
        println!("Rendered");
    }, || {
        renderer.borrow().is_closed()
    });

    machine.run();
}
