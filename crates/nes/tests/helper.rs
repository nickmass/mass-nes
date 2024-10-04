use nes::{Cartridge, Controller, Machine, Region, UserInput};

use std::fs::File;
use std::path::PathBuf;

pub fn rom<P: Into<PathBuf>>(path: P) -> MachineBuilder {
    MachineBuilder {
        rom: path.into(),
        power_up_pc: None,
        debug_mem: None,
        region: Region::Ntsc,
    }
}

pub struct MachineBuilder {
    rom: PathBuf,
    power_up_pc: Option<u16>,
    debug_mem: Option<(u16, u16)>,
    region: Region,
}

impl MachineBuilder {
    fn build(self) -> Machine {
        let mut path = PathBuf::from("tests/data");
        path.push(self.rom);
        let mut file = File::open(path).unwrap();
        let cart = Cartridge::load(&mut file).unwrap();
        let mut machine = Machine::new(self.region, cart);
        if let Some(pc) = self.power_up_pc {
            machine.force_power_up_pc(pc);
        }

        if let Some((addr, size)) = self.debug_mem {
            machine.with_debug_mem(addr, size);
        }

        machine
    }

    pub fn with_debug_mem(mut self, addr: u16, size_kb: u16) -> Self {
        self.debug_mem = Some((addr, size_kb));
        self
    }

    pub fn with_power_up_pc(mut self, pc: u16) -> Self {
        self.power_up_pc = Some(pc);
        self
    }

    pub fn with_region(mut self, region: Region) -> Self {
        self.region = region;
        self
    }
}

impl<'a> Into<MachineBuilder> for &'a str {
    fn into(self) -> MachineBuilder {
        MachineBuilder {
            rom: self.into(),
            power_up_pc: None,
            debug_mem: None,
            region: Region::Ntsc,
        }
    }
}

impl Into<MachineBuilder> for String {
    fn into(self) -> MachineBuilder {
        MachineBuilder {
            rom: self.into(),
            power_up_pc: None,
            debug_mem: None,
            region: Region::Ntsc,
        }
    }
}

pub struct Evaluation {
    condition: Condition,
    message: Option<Message>,
}

impl Evaluation {
    fn assert(&self, machine: &Machine) {
        let message = if let Some(msg) = &self.message {
            let msg_addr = match *msg {
                Message::Absolute(a) => a,
                Message::Indirect(a) => {
                    let low = machine.peek(a) as u16;
                    let high = (machine.peek(a + 1) as u16) << 8;
                    low | high
                }
            };
            let mut msg = String::new();
            let mut msg_offset = 0;
            loop {
                let c = machine.peek(msg_addr + msg_offset) & 0x7f;
                if c == 0 {
                    break;
                }

                msg.push(c as char);
                msg_offset += 1;

                if msg_offset > 256 {
                    break;
                }
            }

            Some(msg)
        } else {
            None
        };

        self.condition.assert(machine, message.as_deref());
    }
}

enum Message {
    Absolute(u16),
    Indirect(u16),
}

#[derive(Debug, Copy, Clone)]
pub enum Condition {
    Equals(u16, u8),
}

impl Condition {
    pub fn with_message(self, addr: u16) -> Evaluation {
        Evaluation {
            condition: self,
            message: Some(Message::Absolute(addr)),
        }
    }

    pub fn with_indirect_message(self, addr: u16) -> Evaluation {
        Evaluation {
            condition: self,
            message: Some(Message::Indirect(addr)),
        }
    }

    fn assert(&self, machine: &Machine, message: Option<&str>) {
        match *self {
            Condition::Equals(addr, expected) => {
                let found = machine.peek(addr);

                if let Some(msg) = message {
                    assert!(
                        expected == found,
                        "{}\nExpected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                        msg,
                        addr,
                        expected,
                        found,
                    );
                } else {
                    assert!(
                        expected == found,
                        "Expected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                        addr,
                        expected,
                        found,
                    );
                }
            }
        }
    }
}

impl Into<Evaluation> for Condition {
    fn into(self) -> Evaluation {
        Evaluation {
            condition: self,
            message: None,
        }
    }
}

pub struct End {
    until: RunUntil,
    frame_limit: Option<u32>,
    test_running: bool,
}

impl End {
    fn done(&mut self, machine: &Machine) -> bool {
        let debug = machine.get_debug();
        let nes_frame = debug.frame(&machine);
        match self.until {
            RunUntil::Frame(frame) => nes_frame >= frame,
            RunUntil::NotEqual(address, value) => {
                let nes_val = machine.peek(address);
                if nes_val == value {
                    self.test_running = true;
                }
                if self.frame_limit.map(|l| nes_frame >= l).unwrap_or(false) {
                    panic!(
                        "hit frame limit of {} before end condition reached",
                        nes_frame
                    );
                }
                self.test_running && value != nes_val
            }
        }
    }
}

pub enum RunUntil {
    Frame(u32),
    NotEqual(u16, u8),
}

impl RunUntil {
    pub fn with_frame_limit(self, frames: u32) -> End {
        let mut end: End = self.into();
        end.frame_limit = Some(frames);
        end
    }
}

impl Into<End> for RunUntil {
    fn into(self) -> End {
        End {
            until: self,
            frame_limit: None,
            test_running: false,
        }
    }
}

pub fn run<M: Into<MachineBuilder>, C: Into<Evaluation>, U: Into<End>>(
    rom: M,
    run_until: U,
    condition: C,
) {
    let eval = condition.into();
    let mut machine = rom.into().build();
    let mut end = run_until.into();

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

        let done = end.done(&machine);

        if done {
            eval.assert(&machine);
            break;
        }
    }
}
