use nes::system::{System, SystemState};
use nes::bus::BusKind;

#[derive(Default)]
pub struct InputState {
    read_counter: u32,
    read_shifter: u8,
    input_buffer: u8,
    pub input: u8,
}

pub trait InputDevice {
    fn to_byte(&self) -> u8;
}

pub struct Controller {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl Controller {
    pub fn new() -> Controller {
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
    }
}

impl InputDevice for Controller {
    fn to_byte(&self) -> u8 {
        let mut value = 0;
        if self.a { value |= 0x01; }
        if self.b { value |= 0x02; }
        if self.select { value |= 0x04; }
        if self.start { value |= 0x08; }
        if self.up { value |= 0x10; }
        if self.down && !self.up { value |= 0x20; }
        if self.left && !self.right { value |= 0x40; }
        if self.right { value |= 0x80; }

        value
    }
}


pub struct Input;

impl Input {
    pub fn new() -> Input {
        Input {}
    }

    pub fn peek(&self, device: BusKind, system: &System, state: &SystemState, addr: u16)-> u8 {
        match addr {
            0x4016 => {
                if state.input.read_counter == 0 {
                    0x41
                } else {
                    let value = (state.input.read_shifter & 1) | 0x40;
                    value
                }
            },
            0x4017 => {
                0x40
            },
            _ => unimplemented!()
        }
    }

    pub fn read(&self, device: BusKind, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match addr {
            0x4016 => {
                if state.input.read_counter == 0 {
                    0x41
                } else {
                    let value = (state.input.read_shifter & 1) | 0x40;
                    state.input.read_shifter >>= 1;
                    state.input.read_counter -= 1;
                    value
                }
            },
            0x4017 => {
                0x40
            },
            _ => unimplemented!()
        }
    }


    pub fn write(&self, device: BusKind, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match addr {
            0x4016 => {
                if value & 0x01 == 1 {
                    state.input.input_buffer = state.input.input;
                } else {
                    state.input.read_shifter = state.input.input_buffer;
                    state.input.read_counter = 8;
                }
            },
            _ => unimplemented!()
        }
    }
}


