#[macro_use]
extern crate glium;

mod nes;
use nes::{Machine, Cartridge, Region};

mod ui;
use ui::gfx::GliumRenderer;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut file = ::std::fs::File::open("/home/nickmass/Downloads/nestest.nes").unwrap();

    let cart = Cartridge::load(&mut file).unwrap();
    
    let renderer = Rc::new(RefCell::new(GliumRenderer::new()));
    let mut machine = Machine::new(Region::Ntsc, cart, |screen| {
        renderer.borrow_mut().render(screen);
        println!("Rendered");
    }, || {
        renderer.borrow().is_closed()
    });

    machine.run();
}
