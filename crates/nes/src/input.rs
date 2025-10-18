#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::{
    MapperInput,
    bus::{Address, AddressBus, AndEqualsAndMask, DeviceKind},
};

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
    did_stobe: bool,
    read_counter: [u32; 2],
    read_shifter: [u8; 2],
    input_buffer: [u8; 2],
}

impl Input {
    pub fn new() -> Input {
        Input {
            ..Default::default()
        }
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Input, AndEqualsAndMask(0xf01f, 0x4016, 0x4016));
        cpu.register_read(DeviceKind::Input, AndEqualsAndMask(0xf01f, 0x4017, 0x4017));
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
            _ => open_bus,
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
            _ => open_bus,
        };

        value | (open_bus & 0xe0)
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4016 => {
                if value & 0x01 == 1 {
                    self.strobe = true;
                    self.did_stobe = false;
                } else {
                    self.strobe = false;
                    self.read_shifter = self.input_buffer;
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn tick<I: InputSource>(&mut self, input_source: &mut I) {
        self.current_tick += 1;
        if self.strobe && self.current_tick & 1 == 0 {
            if !self.did_stobe {
                let (p1, p2) = input_source.strobe();
                self.input_buffer = [p1.to_byte(), p2.to_byte()];
                self.did_stobe = true;
            }
            self.read_counter = [8, 8];
        }
    }
}

pub trait InputSource {
    fn strobe(&mut self) -> (Controller, Controller);
    fn power(&mut self) -> bool;
    fn reset(&mut self) -> bool;
    fn mapper(&mut self) -> Option<MapperInput>;
}

pub struct SimpleInput {
    power: bool,
    reset: bool,
    mapper: Option<MapperInput>,
    player_one: Controller,
    player_two: Controller,
}

#[derive(Debug, Copy, Clone)]
pub enum UserInput {
    PlayerOne(Controller),
    PlayerTwo(Controller),
    Mapper(MapperInput),
    Power,
    Reset,
}

impl SimpleInput {
    pub fn new() -> Self {
        Self {
            power: false,
            reset: false,
            mapper: None,
            player_one: Controller::default(),
            player_two: Controller::default(),
        }
    }

    pub fn handle_input(&mut self, input: crate::UserInput) {
        match input {
            crate::UserInput::PlayerOne(controller) => self.player_one = controller,
            crate::UserInput::PlayerTwo(controller) => self.player_two = controller,
            crate::UserInput::Mapper(mapper_input) => self.mapper = Some(mapper_input),
            crate::UserInput::Power => self.power = true,
            crate::UserInput::Reset => self.reset = true,
        }
    }
}

impl InputSource for SimpleInput {
    fn strobe(&mut self) -> (Controller, Controller) {
        (self.player_one, self.player_two)
    }

    fn power(&mut self) -> bool {
        let power = self.power;
        self.power = false;
        power
    }

    fn reset(&mut self) -> bool {
        let reset = self.reset;
        self.reset = false;
        reset
    }

    fn mapper(&mut self) -> Option<MapperInput> {
        self.mapper.take()
    }
}
