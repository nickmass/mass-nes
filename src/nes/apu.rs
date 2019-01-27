use crate::bus::AddressBus;
use crate::channel::{Channel, Dmc, Noise, Pulse, PulseChannel, Triangle};
use crate::system::{Region, SystemState};

use std::cell::RefCell;

//TODO - Is this table the same for both PAL and NTSC?
pub const LENGTH_TABLE: [u8; 0x20] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Copy, Clone, PartialEq, Eq)]
enum SequenceMode {
    FourStep,
    FiveStep,
}

pub struct ApuState {
    current_tick: u32,
    reset_delay: u32,
    frame_counter: u32,
    sequence_mode: SequenceMode,
    irq_inhibit: bool,
    irq: bool,
    last_4017: u8,
    region: Region,
}

impl Default for ApuState {
    fn default() -> ApuState {
        ApuState {
            current_tick: 0,
            reset_delay: 0,
            frame_counter: 6,
            sequence_mode: SequenceMode::FourStep,
            irq_inhibit: false,
            irq: false,
            last_4017: 0,
            region: Region::default(),
        }
    }
}

impl ApuState {
    fn new(region: Region) -> ApuState {
        ApuState {
            region: region,
            ..Default::default()
        }
    }
    fn sequence_steps(&self) -> &'static [u32] {
        match self.sequence_mode {
            SequenceMode::FourStep => self.region.four_step_seq(),
            SequenceMode::FiveStep => self.region.five_step_seq(),
        }
    }

    pub fn is_quarter_frame(&self) -> bool {
        let steps = self.sequence_steps();
        self.frame_counter == steps[0]
            || self.frame_counter == steps[1]
            || self.frame_counter == steps[2]
            || self.frame_counter == steps[3]
    }

    pub fn is_half_frame(&self) -> bool {
        let steps = self.sequence_steps();
        self.frame_counter == steps[1] || self.frame_counter == steps[3]
    }

    fn is_irq_frame(&self) -> bool {
        match self.sequence_mode {
            SequenceMode::FourStep => {
                let steps = self.sequence_steps();
                !self.irq_inhibit
                    && (self.frame_counter == steps[3] - 1
                        || self.frame_counter == steps[3]
                        || self.frame_counter == 0)
            }
            SequenceMode::FiveStep => false,
        }
    }

    fn increment_frame_counter(&mut self) {
        self.frame_counter += 1;
        let steps = self.sequence_steps();
        if self.frame_counter == steps[4] {
            self.frame_counter = 0;
        }
    }
}

pub struct Apu {
    pub pulse_one: Pulse,
    pub pulse_two: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
    pub dmc: Dmc,
    pulse_table: Vec<i16>,
    tnd_table: Vec<i16>,
    region: Region,
    state: RefCell<ApuState>,
    samples: Vec<i16>,
    sample_index: usize,
}

impl Apu {
    pub fn new(region: Region) -> Apu {
        let mut pulse_table = Vec::new();
        for x in 0..32 {
            let f_val = 95.52 / (8128.0 / (x as f64) + 100.0);
            pulse_table.push(((f_val - 0.5) * ::std::i16::MAX as f64) as i16);
        }

        let mut tnd_table = Vec::new();
        for x in 0..204 {
            let f_val = 163.67 / (24329.0 / (x as f64) + 100.0);
            tnd_table.push(((f_val - 0.5) * ::std::i16::MAX as f64) as i16);
        }

        Apu {
            pulse_one: Pulse::new(PulseChannel::InternalOne, region),
            pulse_two: Pulse::new(PulseChannel::InternalTwo, region),
            triangle: Triangle::new(region),
            noise: Noise::new(region),
            dmc: Dmc::new(region),
            pulse_table: pulse_table,
            tnd_table: tnd_table,
            region: region,
            state: RefCell::new(ApuState::new(region)),
            samples: vec![0; 33248], //Max cycles for the longer pal frame
            sample_index: 0,
        }
    }

    pub fn power(&mut self) {
        for a in 0..4 {
            self.pulse_one.write(a, 0);
            self.pulse_two.write(a, 0);
            self.noise.write(a, 0);
            self.triangle.write(a, 0);
        }
        self.write(0x4015, 0);
        self.write(0x4017, 0);
        self.state.borrow_mut().reset_delay = 6;
    }

    pub fn reset(&mut self) {
        self.write(0x4015, 0);
        let val = self.state.borrow().last_4017;
        self.write(0x4017, val);
        self.state.borrow_mut().reset_delay = 6;
    }

    pub fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut val = 0;
                if self.pulse_one.get_state() {
                    val |= 0x01;
                }
                if self.pulse_two.get_state() {
                    val |= 0x02;
                }
                if self.triangle.get_state() {
                    val |= 0x04;
                }
                if self.noise.get_state() {
                    val |= 0x08;
                }
                if self.dmc.get_state() {
                    val |= 0x10;
                }
                if self.state.borrow().irq {
                    val |= 0x40;
                }
                if self.dmc.get_irq() {
                    val |= 0x80;
                }
                val
            }
            _ => unreachable!(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let mut state = self.state.borrow_mut();
        match addr {
            0x4015 => {
                let mut val = 0;
                if self.pulse_one.get_state() {
                    val |= 0x01;
                }
                if self.pulse_two.get_state() {
                    val |= 0x02;
                }
                if self.triangle.get_state() {
                    val |= 0x04;
                }
                if self.noise.get_state() {
                    val |= 0x08;
                }
                if self.dmc.get_state() {
                    val |= 0x10;
                }
                if state.irq {
                    val |= 0x40;
                }
                if self.dmc.get_irq() {
                    val |= 0x80;
                }
                state.irq = false;
                val
            }
            _ => unreachable!(),
        }
    }

    pub fn write(&self, addr: u16, value: u8) {
        let mut state = self.state.borrow_mut();
        match addr {
            0x4015 => {
                if value & 1 != 0 {
                    self.pulse_one.enable();
                } else {
                    self.pulse_one.disable();
                }
                if value & 0x2 != 0 {
                    self.pulse_two.enable();
                } else {
                    self.pulse_two.disable();
                }
                if value & 0x4 != 0 {
                    self.triangle.enable();
                } else {
                    self.triangle.disable();
                }
                if value & 0x8 != 0 {
                    self.noise.enable();
                } else {
                    self.noise.disable();
                }
                if value & 0x10 != 0 {
                    self.dmc.enable();
                } else {
                    self.dmc.disable();
                }
            }
            0x4017 => {
                state.last_4017 = value;
                state.sequence_mode = match value & 0x80 {
                    0 => SequenceMode::FourStep,
                    _ => SequenceMode::FiveStep,
                };
                state.irq_inhibit = value & 0x40 != 0;
                if state.irq_inhibit {
                    state.irq = false
                }
                if state.sequence_mode == SequenceMode::FiveStep {
                    self.forced_clock();
                }
                state.reset_delay = if state.current_tick & 1 == 0 { 3 } else { 4 };
            }
            _ => unreachable!(),
        }
    }

    fn forced_clock(&self) {
        self.pulse_one.forced_clock();
        self.pulse_two.forced_clock();
        self.triangle.forced_clock();
        self.noise.forced_clock();
    }

    pub fn tick(&mut self) {
        let mut state = self.state.borrow_mut();
        state.current_tick += 1;
        state.increment_frame_counter();
        if state.is_irq_frame() {
            state.irq = true;
        }

        if state.reset_delay != 0 {
            state.reset_delay -= 1;
            if state.reset_delay == 0 {
                state.frame_counter = 0;
            }
        }

        let pulse1 = self.pulse_one.tick(&state);
        let pulse2 = self.pulse_two.tick(&state);
        let triangle = self.triangle.tick(&state);
        let noise = self.noise.tick(&state);
        let dmc = self.dmc.tick(&state);

        let pulse_out = self.pulse_table[(pulse1 + pulse2) as usize];
        let tnd_out = self.tnd_table[((3 * triangle) + (2 * noise) + dmc) as usize];

        if let Some(v) = self.samples.get_mut(self.sample_index) {
            *v = pulse_out + tnd_out;
        }

        self.sample_index += 1;
    }

    pub fn get_irq(&self) -> bool {
        self.state.borrow().irq | self.dmc.get_irq()
    }

    pub fn get_dmc_req(&self) -> Option<u16> {
        self.dmc.get_dmc_req()
    }

    pub fn get_samples<'a>(&'a mut self) -> &[i16] {
        let index = self.sample_index;
        self.sample_index = 0;
        &self.samples[0..index]
    }

    pub fn register(&self, state: &mut SystemState, cpu: &mut AddressBus) {
        self.pulse_one.register(state, cpu);
        self.pulse_two.register(state, cpu);
        self.triangle.register(state, cpu);
        self.noise.register(state, cpu);
        self.dmc.register(state, cpu);
    }
}
