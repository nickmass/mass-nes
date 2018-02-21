extern crate nes;
use self::nes::{UserInput, Machine, Region, Cartridge, Controller};

use std::fs::File;
use std::path::Path;
use std::convert::AsRef;

#[derive(Debug, Copy, Clone)]
pub enum Condition {
    Equals(u16, u8),
}

pub fn run<T>(rom: T, frames: u32, condition: Condition) where T: AsRef<str> {
    let mut path = Path::new("./tests/").to_path_buf();
    path.push(Path::new(rom.as_ref()));
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();
    let mut machine = Machine::new(Region::Ntsc, cart);

    loop {
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
        machine.set_input(r);

        {
            let (system, state) = machine.get_debug();
            let nes_frame = system.debug.frame(state);
            if nes_frame >= frames {
                match condition {
                    Condition::Equals(a, v) => {
                        let nes_val = system.debug.peek(system, state, a);
                        assert!(v == nes_val,
                                "Expected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                                a, v, nes_val);
                    }
                }
                break;
            }
        }

        machine.run();
    }
}
