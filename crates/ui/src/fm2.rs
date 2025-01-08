use nes::{Controller, UserInput};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Result as IoResult};

#[derive(Debug, Clone)]
pub struct Fm2Input {
    inputs: VecDeque<nes::UserInput>,
}

impl Fm2Input {
    pub fn read<R: std::io::Read>(reader: R) -> IoResult<Self> {
        let buf_reader = BufReader::new(reader);
        let mut inputs = VecDeque::new();

        inputs.push_back(UserInput::Power);

        for line in buf_reader.split(b'\n') {
            let line = line?;
            if line.get(0).copied() != Some(b'|') {
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
            let Some(_port1) = splits.next() else {
                continue;
            };
            let Some(_exp) = splits.next() else {
                continue;
            };

            if command & 1 != 0 {
                inputs.push_back(UserInput::Reset);
            }
            if command & 2 != 0 {
                inputs.push_back(UserInput::Power);
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
                inputs.push_back(UserInput::PlayerOne(controller));
            }
        }

        Ok(Fm2Input { inputs })
    }
}

impl Iterator for Fm2Input {
    type Item = UserInput;

    fn next(&mut self) -> Option<Self::Item> {
        self.inputs.pop_front()
    }
}
