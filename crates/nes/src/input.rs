use crate::bus::{Address, AddressBus, DeviceKind};

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

#[derive(Default)]
pub struct Input {
    read_counter: u32,
    read_shifter: u8,
    input_buffer: u8,
    input: u8,
}

impl Input {
    pub fn new() -> Input {
        Input {
            ..Default::default()
        }
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Input, Address(0x4016));
        cpu.register_read(DeviceKind::Input, Address(0x4017));
        cpu.register_write(DeviceKind::Input, Address(0x4016));
    }

    pub fn peek(&self, addr: u16, open_bus: u8) -> u8 {
        let value = match addr {
            0x4016 => {
                if self.read_counter == 0 {
                    0x01
                } else {
                    self.read_shifter & 1
                }
            }
            0x4017 => 0x00,
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn read(&mut self, addr: u16, open_bus: u8) -> u8 {
        let value = match addr {
            0x4016 => {
                if self.read_counter == 0 {
                    0x01
                } else {
                    let value = self.read_shifter & 1;
                    self.read_shifter >>= 1;
                    self.read_counter -= 1;
                    value
                }
            }
            0x4017 => 0x00,
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4016 => {
                if value & 0x01 == 1 {
                    self.input_buffer = self.input;
                } else {
                    self.read_shifter = self.input_buffer;
                    self.read_counter = 8;
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn set_input(&mut self, input: u8) {
        self.input = input;
    }
}
