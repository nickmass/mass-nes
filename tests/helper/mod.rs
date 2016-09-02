extern crate mass_nes;
use self::mass_nes::nes::{Machine, Region, Cartridge, Controller};

use std::fs::File;
use std::path::Path;
use std::convert::AsRef;
use std::sync::Mutex;

#[derive(Debug, Copy, Clone)]
pub enum Condition {
    Equals(u16, u8),
}

pub fn run<T>(rom: T, frames: u32, condition: Condition) where T: AsRef<str> {
    let mut path = Path::new("./tests/").to_path_buf();
    path.push(Path::new(rom.as_ref()));
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();
    
    let closed = Mutex::new(false);

    let mut machine = Machine::new(Region::Ntsc, cart,
    |_| {
    },
    |_| {
    },
    || {
        *closed.lock().unwrap()
    },
    || {
        Controller {
            a: false,
            b: false,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
        }
    },
    |system, state| {
        let mut closed = closed.lock().unwrap();
        let nes_frame = system.debug.frame(state);
        *closed = nes_frame > frames;
        if nes_frame >= frames {
            match condition {
                Condition::Equals(a, v) => {
                    let nes_val = system.debug.peek(system, state, a);
                    assert!(v == nes_val, 
                            "Expected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                            a, v, nes_val);
                }
            }
        }
    });

    machine.run();
}
