use nes::{Cartridge, Controller, Machine, Region, UserInput};

use std::convert::AsRef;
use std::fs::File;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Condition {
    Equals(u16, u8),
    PowerUpPc(u16),
}

#[allow(dead_code)]
pub enum RunUntil {
    Frame(u32),
    NotEqual(u16, u8),
}

pub fn run<T>(rom: T, run_until: RunUntil, condition: Condition)
where
    T: AsRef<str>,
{
    let mut path = Path::new("tests/data").to_path_buf();
    path.push(Path::new(rom.as_ref()));
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();
    let mut machine = Machine::new(Region::Ntsc, cart);

    if let Condition::PowerUpPc(addr) = condition {
        machine.force_power_up_pc(addr);
    }

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
        machine.run();

        let debug = machine.get_debug();
        let nes_frame = debug.frame(&machine);
        let done = match run_until {
            RunUntil::Frame(frame) => nes_frame >= frame,
            RunUntil::NotEqual(address, value) => {
                let nes_val = machine.peek(address);
                value != nes_val
            }
        };
        if done {
            match condition {
                Condition::Equals(a, v) => {
                    let nes_val = machine.peek(a);
                    assert!(
                        v == nes_val,
                        "Expected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                        a,
                        v,
                        nes_val,
                    );
                }
                _ => (),
            }
            break;
        }
    }
}
