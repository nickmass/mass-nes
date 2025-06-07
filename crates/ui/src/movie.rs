use nes::{Controller, UserInput};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Result as IoResult, Seek};

#[derive(Debug, Clone)]
pub enum MovieInput {
    Input(UserInput),
    Frame,
}

#[derive(Debug, Clone)]
pub enum MovieFile {
    Fm2(Fm2Input),
    Bk2(Bk2Input),
}

impl MovieFile {
    pub fn fm2<R: Read>(reader: R, offset: i32) -> IoResult<Self> {
        let fm2 = Fm2Input::read(reader, offset)?;
        Ok(MovieFile::Fm2(fm2))
    }

    pub fn bk2<R: Read + Seek>(reader: R, offset: i32) -> IoResult<Self> {
        let bk2 = Bk2Input::read(reader, offset)?;
        Ok(MovieFile::Bk2(bk2))
    }
}

impl Iterator for MovieFile {
    type Item = MovieInput;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MovieFile::Fm2(fm2) => fm2.inputs.pop_front(),
            MovieFile::Bk2(bk2) => bk2.inputs.pop_front(),
        }
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

        while offset > 0 {
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(
                Controller::default(),
            )));
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

        while offset > 0 {
            inputs.push_back(MovieInput::Input(UserInput::PlayerOne(
                Controller::default(),
            )));
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
