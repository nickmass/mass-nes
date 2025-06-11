use nes::{Cartridge, Machine, Region, UserInput};

use std::fs::File;
use std::path::PathBuf;

pub enum Input {
    Delay(u32),
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
    None,
}

pub fn rom<P: Into<PathBuf>>(path: P) -> MachineBuilder {
    MachineBuilder {
        rom: path.into(),
        power_up_pc: None,
        debug_mem: None,
        region: Region::Ntsc,
        input: Vec::new(),
    }
}

pub struct MachineBuilder {
    rom: PathBuf,
    power_up_pc: Option<u16>,
    debug_mem: Option<(u16, u16)>,
    region: Region,
    input: Vec<Input>,
}

impl MachineBuilder {
    fn build(self) -> Machine {
        let mut path = PathBuf::from("tests/data");
        path.push(self.rom);

        eprintln!("cargo run -p mass_nes -- run '{}'", path.display());

        let mut file = File::open(path).unwrap();
        let cart = Cartridge::load(&mut file, None, None, "test.nes").unwrap();
        let mut machine = Machine::new(self.region, cart);
        if let Some(pc) = self.power_up_pc {
            machine.force_power_up_pc(pc);
        }

        if let Some((addr, size)) = self.debug_mem {
            machine.with_debug_mem(addr, size);
        }

        for i in self.input {
            match i {
                Input::Delay(n) => {
                    for _ in 0..n {
                        machine.run();
                    }
                }
                Input::Select => {
                    let mut input = nes::Controller::new();
                    input.select = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::Start => {
                    let mut input = nes::Controller::new();
                    input.start = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::Up => {
                    let mut input = nes::Controller::new();
                    input.up = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::Down => {
                    let mut input = nes::Controller::new();
                    input.down = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::Left => {
                    let mut input = nes::Controller::new();
                    input.left = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::Right => {
                    let mut input = nes::Controller::new();
                    input.right = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::A => {
                    let mut input = nes::Controller::new();
                    input.a = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::B => {
                    let mut input = nes::Controller::new();
                    input.b = true;
                    machine.handle_input(UserInput::PlayerOne(input));
                }
                Input::None => {
                    let input = nes::Controller::new();
                    machine.handle_input(UserInput::PlayerOne(input));
                }
            }
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

    pub fn with_input<I: IntoIterator<Item = Input>>(mut self, input: I) -> Self {
        self.input = input.into_iter().collect();
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
            input: Vec::new(),
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
            input: Vec::new(),
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

            msg.push('\n');

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

#[derive(Debug, Clone)]
pub enum Condition {
    Equals(u16, u8),
    Any(u16, Vec<u8>),
    ScreenCrc(u32),
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

                assert!(
                    expected == found,
                    "{}Expected '0x{:04X}' to be '0x{:02X}', found '0x{:02X}'.",
                    message.unwrap_or(""),
                    addr,
                    expected,
                    found,
                );
            }
            Condition::Any(addr, ref values) => {
                let found = machine.peek(addr);

                let res = values.iter().any(|v| *v == found);

                if !res {
                    use std::fmt::Write;
                    let mut error_values = String::new();
                    for v in values {
                        if error_values.len() == 0 {
                            let _ = write!(error_values, "[0x{:02X}", v);
                        } else {
                            let _ = write!(error_values, ", 0x{:02X}", v);
                        }
                    }
                    error_values.push(']');

                    panic!(
                        "{}Expected '0x{:04X}' to be any of {}, found '0x{:02X}'.",
                        message.unwrap_or(""),
                        addr,
                        error_values,
                        found,
                    );
                }
            }
            Condition::ScreenCrc(expected) => {
                let screen = machine.get_screen();
                let bytes = screen.into_iter().flat_map(|p| p.to_be_bytes());
                let found = crc_32(bytes);

                if expected != found {
                    print_ansi_screen(screen);
                }

                assert!(
                    expected == found,
                    "{}Expected screen CRC-32 to be '{:08X}', found '{:08X}'.",
                    message.unwrap_or(""),
                    expected,
                    found,
                );
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
        machine.run();

        if end.done(&machine) {
            eval.assert(&machine);
            break;
        }
    }
}

#[allow(unused)]
fn print_mem_range(range: std::ops::Range<u16>, machine: &Machine) {
    for addr in range {
        if addr % 16 == 0 {
            println!();
            print!("0x{addr:04x}:");
        }

        let val = machine.peek(addr);

        print!(" 0x{val:02x}");
    }
}

fn print_ansi_screen(screen: &[u16]) {
    use std::io::Write;

    let palette = nes::Region::Ntsc.default_palette();
    let mut stdout = std::io::stdout().lock();

    for row_pair in 0..120 {
        let row_pair = row_pair * 2;
        for x in 0..256 {
            let idx_top = (row_pair) * 256 + x;
            let idx_bot = (row_pair + 1) * 256 + x;
            let pixel_top = (screen[idx_top] as usize) * 3;
            let pixel_bot = (screen[idx_bot] as usize) * 3;

            let r_top = palette[pixel_top + 0];
            let g_top = palette[pixel_top + 1];
            let b_top = palette[pixel_top + 2];

            let r_bot = palette[pixel_bot + 0];
            let g_bot = palette[pixel_bot + 1];
            let b_bot = palette[pixel_bot + 2];

            let _ = write!(
                stdout,
                "\x1b[38;2;{r_top};{g_top};{b_top};48;2;{r_bot};{g_bot};{b_bot}m▀\x1b[m"
            );
        }
        let _ = writeln!(stdout);
    }
}

fn crc_32<I: IntoIterator<Item = u8>>(data: I) -> u32 {
    let mut crc = 0xffffffff;

    for b in data {
        let index = (crc ^ b as u32) & 0xff;
        crc = (crc >> 8) ^ CRC_TABLE[index as usize];
    }

    crc ^= 0xffffffff;
    crc
}

const CRC_TABLE: [u32; 256] = [
    0x00000000, 0x09073096, 0x120e612c, 0x1b0951ba, 0xff6dc419, 0xf66af48f, 0xed63a535, 0xe46495a3,
    0xfedb8832, 0xf7dcb8a4, 0xecd5e91e, 0xe5d2d988, 0x01b64c2b, 0x08b17cbd, 0x13b82d07, 0x1abf1d91,
    0xfdb71064, 0xf4b020f2, 0xefb97148, 0xe6be41de, 0x02dad47d, 0x0bdde4eb, 0x10d4b551, 0x19d385c7,
    0x036c9856, 0x0a6ba8c0, 0x1162f97a, 0x1865c9ec, 0xfc015c4f, 0xf5066cd9, 0xee0f3d63, 0xe7080df5,
    0xfb6e20c8, 0xf269105e, 0xe96041e4, 0xe0677172, 0x0403e4d1, 0x0d04d447, 0x160d85fd, 0x1f0ab56b,
    0x05b5a8fa, 0x0cb2986c, 0x17bbc9d6, 0x1ebcf940, 0xfad86ce3, 0xf3df5c75, 0xe8d60dcf, 0xe1d13d59,
    0x06d930ac, 0x0fde003a, 0x14d75180, 0x1dd06116, 0xf9b4f4b5, 0xf0b3c423, 0xebba9599, 0xe2bda50f,
    0xf802b89e, 0xf1058808, 0xea0cd9b2, 0xe30be924, 0x076f7c87, 0x0e684c11, 0x15611dab, 0x1c662d3d,
    0xf6dc4190, 0xffdb7106, 0xe4d220bc, 0xedd5102a, 0x09b18589, 0x00b6b51f, 0x1bbfe4a5, 0x12b8d433,
    0x0807c9a2, 0x0100f934, 0x1a09a88e, 0x130e9818, 0xf76a0dbb, 0xfe6d3d2d, 0xe5646c97, 0xec635c01,
    0x0b6b51f4, 0x026c6162, 0x196530d8, 0x1062004e, 0xf40695ed, 0xfd01a57b, 0xe608f4c1, 0xef0fc457,
    0xf5b0d9c6, 0xfcb7e950, 0xe7beb8ea, 0xeeb9887c, 0x0add1ddf, 0x03da2d49, 0x18d37cf3, 0x11d44c65,
    0x0db26158, 0x04b551ce, 0x1fbc0074, 0x16bb30e2, 0xf2dfa541, 0xfbd895d7, 0xe0d1c46d, 0xe9d6f4fb,
    0xf369e96a, 0xfa6ed9fc, 0xe1678846, 0xe860b8d0, 0x0c042d73, 0x05031de5, 0x1e0a4c5f, 0x170d7cc9,
    0xf005713c, 0xf90241aa, 0xe20b1010, 0xeb0c2086, 0x0f68b525, 0x066f85b3, 0x1d66d409, 0x1461e49f,
    0x0edef90e, 0x07d9c998, 0x1cd09822, 0x15d7a8b4, 0xf1b33d17, 0xf8b40d81, 0xe3bd5c3b, 0xeaba6cad,
    0xedb88320, 0xe4bfb3b6, 0xffb6e20c, 0xf6b1d29a, 0x12d54739, 0x1bd277af, 0x00db2615, 0x09dc1683,
    0x13630b12, 0x1a643b84, 0x016d6a3e, 0x086a5aa8, 0xec0ecf0b, 0xe509ff9d, 0xfe00ae27, 0xf7079eb1,
    0x100f9344, 0x1908a3d2, 0x0201f268, 0x0b06c2fe, 0xef62575d, 0xe66567cb, 0xfd6c3671, 0xf46b06e7,
    0xeed41b76, 0xe7d32be0, 0xfcda7a5a, 0xf5dd4acc, 0x11b9df6f, 0x18beeff9, 0x03b7be43, 0x0ab08ed5,
    0x16d6a3e8, 0x1fd1937e, 0x04d8c2c4, 0x0ddff252, 0xe9bb67f1, 0xe0bc5767, 0xfbb506dd, 0xf2b2364b,
    0xe80d2bda, 0xe10a1b4c, 0xfa034af6, 0xf3047a60, 0x1760efc3, 0x1e67df55, 0x056e8eef, 0x0c69be79,
    0xeb61b38c, 0xe266831a, 0xf96fd2a0, 0xf068e236, 0x140c7795, 0x1d0b4703, 0x060216b9, 0x0f05262f,
    0x15ba3bbe, 0x1cbd0b28, 0x07b45a92, 0x0eb36a04, 0xead7ffa7, 0xe3d0cf31, 0xf8d99e8b, 0xf1deae1d,
    0x1b64c2b0, 0x1263f226, 0x096aa39c, 0x006d930a, 0xe40906a9, 0xed0e363f, 0xf6076785, 0xff005713,
    0xe5bf4a82, 0xecb87a14, 0xf7b12bae, 0xfeb61b38, 0x1ad28e9b, 0x13d5be0d, 0x08dcefb7, 0x01dbdf21,
    0xe6d3d2d4, 0xefd4e242, 0xf4ddb3f8, 0xfdda836e, 0x19be16cd, 0x10b9265b, 0x0bb077e1, 0x02b74777,
    0x18085ae6, 0x110f6a70, 0x0a063bca, 0x03010b5c, 0xe7659eff, 0xee62ae69, 0xf56bffd3, 0xfc6ccf45,
    0xe00ae278, 0xe90dd2ee, 0xf2048354, 0xfb03b3c2, 0x1f672661, 0x166016f7, 0x0d69474d, 0x046e77db,
    0x1ed16a4a, 0x17d65adc, 0x0cdf0b66, 0x05d83bf0, 0xe1bcae53, 0xe8bb9ec5, 0xf3b2cf7f, 0xfab5ffe9,
    0x1dbdf21c, 0x14bac28a, 0x0fb39330, 0x06b4a3a6, 0xe2d03605, 0xebd70693, 0xf0de5729, 0xf9d967bf,
    0xe3667a2e, 0xea614ab8, 0xf1681b02, 0xf86f2b94, 0x1c0bbe37, 0x150c8ea1, 0x0e05df1b, 0x0702ef8d,
];
