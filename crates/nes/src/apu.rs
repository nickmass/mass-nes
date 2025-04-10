#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{Address, AddressBus, DeviceKind};
use crate::channel::{Channel, Dmc, Noise, Pulse, PulseChannel, Triangle};
use crate::cpu::dma::DmcDmaKind;
use crate::mapper::RcMapper;
use crate::region::Region;

pub const LENGTH_TABLE: [u8; 0x20] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SequenceMode {
    FourStep,
    FiveStep,
}

#[derive(Debug, Copy, Clone)]
pub struct ApuSnapshot {
    pub is_half_frame: bool,
    pub is_quarter_frame: bool,
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Apu {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
    #[cfg_attr(feature = "save-states", save(skip))]
    mapper: RcMapper,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub pulse_one: Pulse,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub pulse_two: Pulse,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub triangle: Triangle,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub noise: Noise,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub dmc: Dmc,
    #[cfg_attr(feature = "save-states", save(skip))]
    pulse_table: Vec<i16>,
    #[cfg_attr(feature = "save-states", save(skip))]
    tnd_table: Vec<i16>,
    #[cfg_attr(feature = "save-states", save(skip))]
    samples: Samples,
    current_tick: u32,
    reset_delay: u32,
    frame_counter: u32,
    sequence_mode: SequenceMode,
    irq_inhibit: bool,
    irq: bool,
    last_4017: u8,
    oam_req: Option<u8>,
}

impl Apu {
    pub fn new(region: Region, mapper: RcMapper) -> Apu {
        let mut pulse_table = Vec::new();
        for x in 0..32 {
            let f_val = 95.52 / (8128.0 / (x as f64) + 100.0);
            pulse_table.push((f_val * ::std::i16::MAX as f64) as i16);
        }

        let mut tnd_table = Vec::new();
        for x in 0..204 {
            let f_val = 163.67 / (24329.0 / (x as f64) + 100.0);
            tnd_table.push((f_val * ::std::i16::MAX as f64) as i16);
        }

        Apu {
            region,
            mapper,
            pulse_one: Pulse::new(PulseChannel::InternalOne),
            pulse_two: Pulse::new(PulseChannel::InternalTwo),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(region),
            pulse_table,
            tnd_table,
            samples: Samples::new(33248), //Max cycles for the longer pal frame
            current_tick: 0,
            reset_delay: 0,
            frame_counter: 6,
            sequence_mode: SequenceMode::FourStep,
            irq_inhibit: false,
            irq: false,
            last_4017: 0,
            oam_req: None,
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

        for _ in 0..4 {
            self.tick();
        }
    }

    pub fn reset(&mut self) {
        self.write(0x4015, 0);
        self.write(0x4017, 0);

        for _ in 0..4 {
            self.tick();
        }
    }

    #[cfg(feature = "debugger")]
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
                if self.irq {
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

    pub fn read(&mut self, addr: u16) -> u8 {
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
                if self.irq {
                    val |= 0x40;
                }
                if self.dmc.get_irq() {
                    val |= 0x80;
                }
                self.irq = false;
                val
            }
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x4014 => {
                self.oam_req = Some(value);
            }
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
                self.last_4017 = value;
                self.sequence_mode = match value & 0x80 {
                    0 => SequenceMode::FourStep,
                    _ => SequenceMode::FiveStep,
                };
                self.irq_inhibit = value & 0x40 != 0;
                if self.irq_inhibit {
                    self.irq = false
                }
                if self.sequence_mode == SequenceMode::FiveStep {
                    self.forced_clock();
                }
                self.reset_delay = if self.current_tick & 1 == 0 { 3 } else { 4 };
            }
            _ => unreachable!(),
        }
    }

    fn forced_clock(&mut self) {
        self.pulse_one.forced_clock();
        self.pulse_two.forced_clock();
        self.triangle.forced_clock();
        self.noise.forced_clock();
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.increment_frame_counter();
        if self.is_irq_frame() {
            self.irq = true;
        }

        if self.reset_delay != 0 {
            self.reset_delay -= 1;
            if self.reset_delay == 0 {
                self.frame_counter = 0;
            }
        }

        let snapshot = self.snapshot();
        let pulse1 = self.pulse_one.tick(snapshot);
        let pulse2 = self.pulse_two.tick(snapshot);
        let triangle = self.triangle.tick(snapshot);
        let noise = self.noise.tick(snapshot);
        let dmc = self.dmc.tick(snapshot);

        let pulse_out = self.pulse_table[(pulse1 + pulse2) as usize];
        let tnd_out = self.tnd_table[((3 * triangle) + (2 * noise) + dmc) as usize];
        let ext_out = self.mapper.get_sample().unwrap_or(0);
        let sample = (pulse_out + tnd_out) - ext_out;
        self.samples.push(sample);
    }

    pub fn samples(&mut self) -> impl Iterator<Item = i16> + '_ {
        self.samples.iter()
    }

    pub fn take_samples(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = i16> + ExactSizeIterator + '_ {
        self.samples.take_iter()
    }

    pub fn get_irq(&self) -> bool {
        self.irq | self.dmc.get_irq()
    }

    pub fn get_dmc_req(&mut self) -> Option<DmcDmaKind> {
        self.dmc.get_dmc_req()
    }

    pub fn get_oam_req(&mut self) -> Option<u8> {
        self.oam_req.take()
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Apu, Address(0x4015));
        cpu.register_write(DeviceKind::Apu, Address(0x4014));
        cpu.register_write(DeviceKind::Apu, Address(0x4015));
        cpu.register_write(DeviceKind::Apu, Address(0x4017));

        self.pulse_one.register(cpu);
        self.pulse_two.register(cpu);
        self.triangle.register(cpu);
        self.noise.register(cpu);
        self.dmc.register(cpu);
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
                    && (self.frame_counter == steps[3]
                        || self.frame_counter == steps[3] - 1
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

    fn snapshot(&self) -> ApuSnapshot {
        ApuSnapshot {
            is_half_frame: self.is_half_frame(),
            is_quarter_frame: self.is_quarter_frame(),
        }
    }
}

pub struct Samples {
    samples: Vec<i16>,
    capacity: usize,
    reader_idx: usize,
    writer_idx: usize,
}

impl Samples {
    fn new(capacity: usize) -> Self {
        Self {
            samples: vec![0; capacity],
            capacity,
            reader_idx: 0,
            writer_idx: 0,
        }
    }

    fn push(&mut self, sample: i16) {
        self.samples[self.writer_idx] = sample;
        self.advance_writer();
        if self.reader_idx == self.writer_idx {
            self.advance_reader();
        }
    }

    fn pop(&mut self) -> Option<i16> {
        if self.reader_idx == self.writer_idx {
            None
        } else {
            let sample = self.samples[self.reader_idx];
            self.advance_reader();
            Some(sample)
        }
    }

    fn iter(&mut self) -> impl Iterator<Item = i16> + '_ {
        std::iter::from_fn(|| self.pop())
    }

    fn take_iter(&mut self) -> SamplesIter {
        let iter = SamplesIter {
            samples: &self.samples,
            done: false,
            capacity: self.capacity,
            reader_idx: self.reader_idx,
            writer_idx: self.writer_idx,
        };

        self.reader_idx = self.writer_idx;

        iter
    }

    fn advance_reader(&mut self) {
        self.reader_idx += 1;
        self.reader_idx *= (self.reader_idx < self.capacity) as usize;
    }

    fn advance_writer(&mut self) {
        self.writer_idx += 1;
        self.writer_idx *= (self.writer_idx < self.capacity) as usize;
    }
}

struct SamplesIter<'a> {
    samples: &'a [i16],
    done: bool,
    capacity: usize,
    reader_idx: usize,
    writer_idx: usize,
}

impl<'a> SamplesIter<'a> {
    fn advance_reader(&mut self) {
        self.reader_idx += 1;
        self.reader_idx *= (self.reader_idx < self.capacity) as usize;
    }

    fn retreat_writer(&mut self) {
        if self.writer_idx == 0 {
            self.writer_idx = self.capacity - 1;
        } else {
            self.writer_idx -= 1;
        }
    }
}

impl<'a> Iterator for SamplesIter<'a> {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader_idx == self.writer_idx || self.done {
            self.done = true;
            None
        } else {
            let sample = self.samples[self.reader_idx];
            self.advance_reader();
            Some(sample)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = if self.done {
            0
        } else if self.reader_idx < self.writer_idx {
            self.writer_idx - self.reader_idx
        } else {
            self.capacity - (self.reader_idx - self.writer_idx)
        };

        (len, Some(len))
    }
}

impl<'a> DoubleEndedIterator for SamplesIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else {
            self.retreat_writer();
            if self.reader_idx == self.writer_idx {
                self.done = true;
            }
            Some(self.samples[self.writer_idx])
        }
    }
}

impl<'a> ExactSizeIterator for SamplesIter<'a> {}
