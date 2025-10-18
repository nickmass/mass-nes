use nes::{Controller, UserInput};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Result as IoResult, Seek};

#[derive(Debug, Clone)]
pub enum MovieInput {
    Input(UserInput),
    Frame,
}

#[derive(Debug, Copy, Clone)]
pub enum SubframeMode {
    On,
    Off,
    Auto,
}

#[derive(Debug, Clone)]
pub struct MovieFile {
    subframe: bool,
    inputs: VecDeque<MovieInput>,
    player_one: Controller,
    player_two: Controller,
    reset: bool,
    power: bool,
}

impl MovieFile {
    fn new(inputs: VecDeque<MovieInput>, subframe: bool) -> Self {
        Self {
            subframe,
            inputs,
            player_one: Controller::default(),
            player_two: Controller::default(),
            reset: false,
            power: false,
        }
    }

    pub fn fm2<R: Read>(reader: R, offset: i32, subframe: SubframeMode) -> IoResult<Self> {
        let fm2 = Fm2Input::read(reader, offset)?;

        let subframe = match subframe {
            SubframeMode::On => true,
            SubframeMode::Off => false,
            SubframeMode::Auto => false,
        };
        Ok(MovieFile::new(fm2.inputs, subframe))
    }

    pub fn bk2<R: Read + Seek>(reader: R, offset: i32, subframe: SubframeMode) -> IoResult<Self> {
        let bk2 = Bk2Input::read(reader, offset)?;

        let subframe = match subframe {
            SubframeMode::On => true,
            SubframeMode::Off => false,
            SubframeMode::Auto => false,
        };
        Ok(MovieFile::new(bk2.inputs, subframe))
    }

    pub fn r08<R: Read>(reader: R, offset: i32, subframe: SubframeMode) -> IoResult<Self> {
        let r08 = R08Input::read(reader, offset)?;

        let subframe = match subframe {
            SubframeMode::On => true,
            SubframeMode::Off => false,
            SubframeMode::Auto => true,
        };
        Ok(MovieFile::new(r08.inputs, subframe))
    }

    pub fn prepare_frame(&mut self) {
        if !self.subframe {
            while let Some(input) = self.inputs.pop_front() {
                match input {
                    MovieInput::Input(input) => match input {
                        UserInput::PlayerOne(controller) => self.player_one = controller,
                        UserInput::PlayerTwo(controller) => self.player_two = controller,
                        UserInput::Mapper(_) => (),
                        UserInput::Power => self.power = true,
                        UserInput::Reset => self.reset = true,
                    },
                    MovieInput::Frame => break,
                }
            }
        }
    }

    pub fn done(&self) -> bool {
        self.inputs.is_empty()
    }
}

impl nes::InputSource for MovieFile {
    fn strobe(&mut self) -> (Controller, Controller) {
        if self.subframe {
            let mut player_one = Controller::default();
            let mut player_two = Controller::default();
            while let Some(input) = self.inputs.pop_front() {
                match input {
                    MovieInput::Input(input) => match input {
                        UserInput::PlayerOne(controller) => player_one = controller,
                        UserInput::PlayerTwo(controller) => player_two = controller,
                        UserInput::Mapper(_) => (),
                        UserInput::Power => self.power = true,
                        UserInput::Reset => self.reset = true,
                    },
                    MovieInput::Frame => break,
                }
            }

            (player_one, player_two)
        } else {
            (self.player_one, self.player_two)
        }
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

    fn mapper(&mut self) -> Option<nes::MapperInput> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Fm2Input {
    inputs: VecDeque<MovieInput>,
}

impl Fm2Input {
    pub fn read<R: Read>(reader: R, mut offset: i32) -> IoResult<Self> {
        let buf_reader = BufReader::new(reader);
        let mut inputs = VecDeque::new();

        inputs.push_back(MovieInput::Input(UserInput::Power));
        inputs.push_back(MovieInput::Frame);

        while offset > 0 {
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(
                Controller::default(),
            )));
            inputs.push_back(MovieInput::Frame);
            offset -= 1;
        }

        for line in buf_reader.split(b'\n') {
            let line = line?;
            if line.get(0).copied() != Some(b'|') {
                continue;
            }

            if offset < 0 {
                offset += 1;
                continue;
            }

            let mut splits = line.split(|&b| b == b'|');
            let _ = splits.next();
            let Some(command) = splits
                .next()
                .and_then(|n| std::str::from_utf8(n).ok())
                .and_then(|n| n.parse::<u32>().ok())
            else {
                continue;
            };
            let Some(port0) = splits.next() else {
                continue;
            };
            let Some(port1) = splits.next() else {
                continue;
            };
            let Some(_exp) = splits.next() else {
                continue;
            };

            if command & 1 != 0 {
                inputs.push_back(MovieInput::Input(UserInput::Reset));
            }
            if command & 2 != 0 {
                inputs.push_back(MovieInput::Input(UserInput::Power));
            }

            let is_pressed = |c| c != b'.' && c != b' ';

            if port0.len() == 8 {
                let port = port0;
                let controller = Controller {
                    right: is_pressed(port[0]),
                    left: is_pressed(port[1]),
                    down: is_pressed(port[2]),
                    up: is_pressed(port[3]),
                    start: is_pressed(port[4]),
                    select: is_pressed(port[5]),
                    b: is_pressed(port[6]),
                    a: is_pressed(port[7]),
                };
                inputs.push_back(MovieInput::Input(UserInput::PlayerOne(controller)));
            }
            if port1.len() == 8 {
                let port = port1;
                let controller = Controller {
                    right: is_pressed(port[0]),
                    left: is_pressed(port[1]),
                    down: is_pressed(port[2]),
                    up: is_pressed(port[3]),
                    start: is_pressed(port[4]),
                    select: is_pressed(port[5]),
                    b: is_pressed(port[6]),
                    a: is_pressed(port[7]),
                };
                inputs.push_back(MovieInput::Input(UserInput::PlayerTwo(controller)));
            }

            inputs.push_back(MovieInput::Frame);
        }

        Ok(Fm2Input { inputs })
    }
}

#[derive(Debug, Clone)]
pub struct Bk2Input {
    inputs: VecDeque<MovieInput>,
}

impl Bk2Input {
    pub fn read<R: Read + Seek>(reader: R, mut offset: i32) -> IoResult<Self> {
        let mut zip = zip::ZipArchive::new(reader)?;
        let file = zip.by_name("Input Log.txt")?;
        let buf_reader = BufReader::new(file);

        let mut inputs = VecDeque::new();

        inputs.push_back(MovieInput::Input(UserInput::Power));
        inputs.push_back(MovieInput::Frame);

        while offset > 0 {
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(
                Controller::default(),
            )));
            inputs.push_back(MovieInput::Frame);
            offset -= 1;
        }

        let mut input = Vec::new();

        for line in buf_reader.split(b'\n') {
            let line = line?;
            if line.get(0).copied() != Some(b'|') {
                continue;
            }

            if offset < 0 {
                offset += 1;
                continue;
            }

            input.clear();

            for c in line {
                if c != b'|' {
                    input.push(c);
                }
            }

            let is_pressed = |idx| {
                if let Some(&c) = input.get(idx) {
                    c != b'.' && c != b' '
                } else {
                    false
                }
            };

            if is_pressed(0) {
                inputs.push_back(MovieInput::Input(UserInput::Power));
            }
            if is_pressed(1) {
                inputs.push_back(MovieInput::Input(UserInput::Power));
            }

            let controller = Controller {
                up: is_pressed(2),
                down: is_pressed(3),
                left: is_pressed(4),
                right: is_pressed(5),
                start: is_pressed(6),
                select: is_pressed(7),
                b: is_pressed(8),
                a: is_pressed(9),
            };
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(controller)));

            if input.len() > 10 {
                let controller = Controller {
                    up: is_pressed(10),
                    down: is_pressed(11),
                    left: is_pressed(12),
                    right: is_pressed(13),
                    start: is_pressed(14),
                    select: is_pressed(15),
                    b: is_pressed(16),
                    a: is_pressed(17),
                };
                inputs.push_back(MovieInput::Input(UserInput::PlayerTwo(controller)));
            }

            inputs.push_back(MovieInput::Frame);
        }

        Ok(Bk2Input { inputs })
    }
}

#[derive(Debug, Clone)]
pub struct R08Input {
    inputs: VecDeque<MovieInput>,
}

impl R08Input {
    pub fn read<R: Read>(reader: R, mut offset: i32) -> IoResult<Self> {
        let buf_reader = BufReader::new(reader);

        let mut inputs = VecDeque::new();

        inputs.push_back(MovieInput::Input(UserInput::Power));
        inputs.push_back(MovieInput::Frame);

        while offset > 0 {
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(
                Controller::default(),
            )));
            inputs.push_back(MovieInput::Frame);
            offset -= 1;
        }

        let mut bytes = buf_reader.bytes();

        loop {
            let Some(p1) = bytes.next() else {
                break;
            };
            let Some(p2) = bytes.next() else {
                break;
            };

            let p1 = p1?;
            let p2 = p2?;

            if offset < 0 {
                offset += 1;
                continue;
            }

            let from_byte = |b: u8| Controller {
                a: b & 0x80 != 0,
                b: b & 0x40 != 0,
                select: b & 0x20 != 0,
                start: b & 0x10 != 0,
                up: b & 0x8 != 0,
                down: b & 0x4 != 0,
                left: b & 0x2 != 0,
                right: b & 0x1 != 0,
            };

            let p1 = from_byte(p1);
            let p2 = from_byte(p2);

            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(p1)));
            inputs.push_back(MovieInput::Input(UserInput::PlayerTwo(p2)));
            inputs.push_back(MovieInput::Frame);
        }

        Ok(R08Input { inputs })
    }
}
