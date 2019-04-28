use std::cell::RefCell;

#[derive(Default)]
pub struct InputState {
    read_counter: u32,
    read_shifter: u8,
    input_buffer: u8,
    input: u8,
}

pub trait InputDevice {
    fn to_byte(&self) -> u8;
}

#[derive(Copy, Clone, Debug)]
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
        if self.a {
            value |= 0x01;
        }
        if self.b {
            value |= 0x02;
        }
        if self.select {
            value |= 0x04;
        }
        if self.start {
            value |= 0x08;
        }
        if self.up {
            value |= 0x10;
        }
        if self.down && !self.up {
            value |= 0x20;
        }
        if self.left && !self.right {
            value |= 0x40;
        }
        if self.right {
            value |= 0x80;
        }

        value
    }
}

pub struct Input {
    state: RefCell<InputState>,
}

impl Input {
    pub fn new() -> Input {
        Input {
            state: RefCell::new(InputState::default()),
        }
    }

    pub fn peek(&self, addr: u16, open_bus: u8) -> u8 {
        let state = self.state.borrow();
        let value = match addr {
            0x4016 => {
                if state.read_counter == 0 {
                    0x01
                } else {
                    state.read_shifter & 1
                }
            }
            0x4017 => 0x00,
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn read(&self, addr: u16, open_bus: u8) -> u8 {
        let mut state = self.state.borrow_mut();
        let value = match addr {
            0x4016 => {
                if state.read_counter == 0 {
                    0x01
                } else {
                    let value = state.read_shifter & 1;
                    state.read_shifter >>= 1;
                    state.read_counter -= 1;
                    value
                }
            }
            0x4017 => 0x00,
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn write(&self, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        match addr {
            0x4016 => {
                if value & 0x01 == 1 {
                    state.input_buffer = state.input;
                } else {
                    state.read_shifter = state.input_buffer;
                    state.read_counter = 8;
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn set_input(&mut self, input: u8) {
        let mut state = self.state.borrow_mut();
        state.input = input;
    }
}
