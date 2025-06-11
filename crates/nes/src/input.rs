#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{Address, AddressBus, DeviceKind};

pub trait InputDevice {
    fn to_byte(&self) -> u8;
}

#[derive(Debug, Copy, Clone, Default)]
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
        if self.down {
            value |= 0x20;
        }
        if self.left {
            value |= 0x40;
        }
        if self.right {
            value |= 0x80;
        }

        value
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Default)]
pub struct Input {
    current_tick: u32,
    strobe: bool,
    read_counter: [u32; 2],
    read_shifter: [u8; 2],
    input_buffer: [u8; 2],
    input: [u8; 2],
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

    #[cfg(feature = "debugger")]
    pub fn peek(&self, addr: u16, open_bus: u8) -> u8 {
        let value = match addr {
            0x4016 => {
                if self.read_counter[0] == 0 {
                    0x01
                } else {
                    self.read_shifter[0] & 1
                }
            }
            0x4017 => {
                if self.read_counter[1] == 0 {
                    0x01
                } else {
                    self.read_shifter[1] & 1
                }
            }
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn read(&mut self, addr: u16, open_bus: u8) -> u8 {
        let value = match addr {
            0x4016 => {
                if self.read_counter[0] == 0 {
                    0x01
                } else {
                    let value = self.read_shifter[0] & 1;
                    self.read_shifter[0] >>= 1;
                    self.read_counter[0] -= 1;
                    value
                }
            }
            0x4017 => {
                if self.read_counter[1] == 0 {
                    0x01
                } else {
                    let value = self.read_shifter[1] & 1;
                    self.read_shifter[1] >>= 1;
                    self.read_counter[1] -= 1;
                    value
                }
            }
            _ => unimplemented!(),
        };

        value | (open_bus & 0xe0)
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4016 => {
                if value & 0x01 == 1 {
                    self.strobe = true;
                } else {
                    self.strobe = false;
                    self.read_shifter = self.input_buffer;
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
        if self.strobe && self.current_tick & 1 == 0 {
            self.input_buffer = self.input;
            self.read_counter = [8, 8];
        }
    }

    pub fn set_port_one(&mut self, port_one: u8) {
        self.input[0] = port_one;
    }

    pub fn set_port_two(&mut self, port_two: u8) {
        self.input[1] = port_two;
    }
}
