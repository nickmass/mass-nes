#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{Address, AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::{Channel, Dmc, Noise, Pulse, PulseChannel, Triangle};
use crate::cpu::dma::DmcDmaKind;
use crate::mapper::RcMapper;
use crate::region::Region;
use crate::ring_buf::RingBuf;
use crate::run_until::{self, RunUntil};

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
pub struct Apu<S: Sample = i16> {
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
    mixer: S::Mixer,
    #[cfg_attr(feature = "save-states", save(skip))]
    samples: RingBuf<S>,
    current_tick: u32,
    reset_delay: u32,
    frame_counter: u32,
    sequence_mode: SequenceMode,
    irq_inhibit: bool,
    irq: bool,
    irq_flag: bool,
    irq_flag_read: bool,
    last_4017: u8,
    oam_req: Option<u8>,
    #[cfg(feature = "debugger")]
    #[cfg_attr(feature = "save-states", save(skip))]
    debug_channels: DebugChannelSamples,
    #[cfg_attr(feature = "save-states", save(skip))]
    playback: ChannelPlayback,
}

impl<S: Sample> Apu<S> {
    pub fn new(region: Region, mapper: RcMapper) -> Apu<S> {
        Apu {
            region,
            mapper,
            pulse_one: Pulse::new(PulseChannel::InternalOne),
            pulse_two: Pulse::new(PulseChannel::InternalTwo),
            triangle: Triangle::new(),
            noise: Noise::new(region),
            dmc: Dmc::new(region),
            mixer: S::Mixer::default(),
            samples: RingBuf::new(region.frame_ticks().ceil() as usize * 2),
            current_tick: 0,
            reset_delay: 0,
            frame_counter: 6,
            sequence_mode: SequenceMode::FourStep,
            irq_inhibit: false,
            irq: false,
            irq_flag: false,
            irq_flag_read: false,
            last_4017: 0,
            oam_req: None,
            #[cfg(feature = "debugger")]
            debug_channels: DebugChannelSamples::new(),
            playback: ChannelPlayback::default(),
        }
    }

    pub fn power(&mut self) {
        self.dmc.power();
        for a in 0..4 {
            self.pulse_one.write(a, 0);
            self.pulse_two.write(a, 0);
            self.noise.write(a, 0);
            self.triangle.write(a, 0);
        }
        self.write(0x4015, 0);
        self.write(0x4017, 0);

        for _ in 0..2 {
            self.tick(&mut run_until::Frames(1));
        }
    }

    pub fn reset(&mut self) {
        self.write(0x4015, 0);
        self.write(0x4017, 0);

        for _ in 0..2 {
            self.tick(&mut run_until::Frames(1));
        }
    }

    #[cfg(feature = "debugger")]
    pub fn watch(&self, visitor: &mut crate::debug::WatchVisitor) {
        let mut apu = visitor.group("APU");
        apu.value("IRQ", self.irq);
        apu.value("Quater Frame", self.is_quarter_frame());
        apu.value("Half Frame", self.is_half_frame());
        let four_step = matches!(self.sequence_mode, SequenceMode::FourStep);
        apu.value("Four Step", four_step);
        self.pulse_one.watch(&mut apu);
        self.pulse_two.watch(&mut apu);
        self.triangle.watch(&mut apu);
        self.noise.watch(&mut apu);
        self.dmc.watch(&mut apu);
    }

    #[cfg(feature = "debugger")]
    pub fn peek(&self, addr: u16, open_bus: u8) -> u8 {
        match addr {
            0x4015 => {
                let mut val = open_bus & 0x20;
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
            _ => open_bus,
        }
    }

    pub fn read(&mut self, addr: u16, open_bus: u8) -> u8 {
        match addr {
            0x4015 => {
                let mut val = open_bus & 0x20;
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
                if self.irq_flag {
                    val |= 0x40;
                }
                if self.dmc.get_irq() {
                    val |= 0x80;
                }
                self.irq = false;
                self.irq_flag_read = true;
                val
            }
            _ => open_bus,
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
                    self.irq = false;
                    self.irq_flag = false;
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

    pub fn tick<U: RunUntil>(&mut self, until: &mut U) {
        self.current_tick += 1;
        self.increment_frame_counter();
        self.trigger_irq();

        if self.irq_flag && self.irq_flag_read && self.current_tick & 1 == 1 {
            self.irq_flag = false;
            self.irq_flag_read = false;
        }

        if self.reset_delay != 0 {
            self.reset_delay -= 1;
            if self.reset_delay == 0 {
                self.frame_counter = 0;
            }
        }

        let snapshot = self.snapshot();
        let pulse_1 = self.pulse_one.tick(snapshot);
        let pulse_2 = self.pulse_two.tick(snapshot);
        let triangle = self.triangle.tick(snapshot);
        let noise = self.noise.tick(snapshot);
        let dmc = self.dmc.tick(snapshot);
        let ext = self.mapper.get_sample().unwrap_or(0);

        #[cfg(feature = "debugger")]
        self.debug_channels
            .push_channels(pulse_1, pulse_2, triangle, noise, dmc, ext);

        let pulse_1 = self.playback.pulse_1(pulse_1);
        let pulse_2 = self.playback.pulse_2(pulse_2);
        let triangle = self.playback.triangle(triangle);
        let noise = self.playback.noise(noise);
        let dmc = self.playback.dmc(dmc);
        let ext = self.playback.ext(ext);

        let sample = self.mixer.mix(pulse_1, pulse_2, triangle, noise, dmc, ext);
        self.samples.push(sample);
        until.add_sample();
    }

    pub fn samples(&mut self) -> impl Iterator<Item = S> + '_ {
        self.samples.iter()
    }

    pub fn take_samples(&mut self) -> impl DoubleEndedIterator<Item = S> + ExactSizeIterator + '_ {
        self.samples.take_iter()
    }

    #[cfg(feature = "debugger")]
    pub fn take_channel_samples(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = ChannelSamples> + ExactSizeIterator + '_ {
        self.debug_channels.samples.take_iter()
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

    pub fn set_channel_playback(&mut self, playback: ChannelPlayback) {
        self.playback = playback;
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Apu, AndEqualsAndMask(0xf01f, 0x4015, 0x4015));
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

    fn trigger_irq(&mut self) {
        if let SequenceMode::FiveStep = self.sequence_mode {
            return;
        }
        let steps = self.sequence_steps();

        if self.frame_counter == steps[3] - 1 || self.frame_counter == steps[3] {
            if !self.irq_inhibit {
                self.irq = true;
            }
            self.irq_flag = true;
            self.irq_flag_read = false;
        } else if self.frame_counter == 0 {
            if !self.irq_inhibit {
                self.irq = true;
            }
            self.irq_flag = self.irq;
            self.irq_flag_read = false;
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

struct DebugChannelSamples {
    window_size: usize,
    sample_accum_count: usize,
    sample_accum: ChannelSamples,
    samples: RingBuf<ChannelSamples>,
}

#[allow(unused)]
impl DebugChannelSamples {
    fn new() -> Self {
        const WINDOW_SIZE: usize = 32;
        DebugChannelSamples {
            window_size: WINDOW_SIZE,
            sample_accum_count: 0,
            sample_accum: ChannelSamples::zero(),
            samples: RingBuf::new((33248 / WINDOW_SIZE) + 1),
        }
    }

    fn push_channels(
        &mut self,
        pulse_1: u8,
        pulse_2: u8,
        triangle: u8,
        noise: u8,
        dmc: u8,
        external: i16,
    ) {
        let channels = ChannelSamples {
            pulse_1: pulse_1 as f32 / 15.0,
            pulse_2: pulse_2 as f32 / 15.0,
            triangle: triangle as f32 / 15.0,
            noise: noise as f32 / 15.0,
            dmc: dmc as f32 / 127.0,
            external: (external as f32).abs() / i16::MAX as f32,
        };

        self.sample_accum += channels;
        self.sample_accum_count += 1;
        if self.sample_accum_count >= self.window_size {
            let channels = self.sample_accum.clone() / self.sample_accum_count as f32;
            self.samples.push(channels);
            self.sample_accum_count = 0;
            self.sample_accum = ChannelSamples::zero();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelSamples {
    pub pulse_1: f32,
    pub pulse_2: f32,
    pub triangle: f32,
    pub noise: f32,
    pub dmc: f32,
    pub external: f32,
}

impl Default for ChannelSamples {
    fn default() -> Self {
        Self::zero()
    }
}

impl ChannelSamples {
    fn zero() -> Self {
        ChannelSamples {
            pulse_1: 0.0,
            pulse_2: 0.0,
            triangle: 0.0,
            noise: 0.0,
            dmc: 0.0,
            external: 0.0,
        }
    }
}

impl std::ops::AddAssign for ChannelSamples {
    fn add_assign(&mut self, rhs: Self) {
        self.pulse_1 += rhs.pulse_1;
        self.pulse_2 += rhs.pulse_2;
        self.triangle += rhs.triangle;
        self.noise += rhs.noise;
        self.dmc += rhs.dmc;
        self.external += rhs.external;
    }
}

impl std::ops::Div<f32> for ChannelSamples {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        ChannelSamples {
            pulse_1: self.pulse_1 / rhs,
            pulse_2: self.pulse_2 / rhs,
            triangle: self.triangle / rhs,
            noise: self.noise / rhs,
            dmc: self.dmc / rhs,
            external: self.external / rhs,
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct ChannelPlayback {
    pub pulse_1_solo: bool,
    pub pulse_2_solo: bool,
    pub triangle_solo: bool,
    pub noise_solo: bool,
    pub dmc_solo: bool,
    pub ext_solo: bool,
    pub pulse_1_mute: bool,
    pub pulse_2_mute: bool,
    pub triangle_mute: bool,
    pub noise_mute: bool,
    pub dmc_mute: bool,
    pub ext_mute: bool,
}

impl ChannelPlayback {
    fn any_solo(&self) -> bool {
        self.pulse_1_solo
            || self.pulse_2_solo
            || self.triangle_solo
            || self.noise_solo
            || self.dmc_solo
            || self.ext_solo
    }

    fn pulse_1(&self, v: u8) -> u8 {
        if self.pulse_1_mute || (!self.pulse_1_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }

    fn pulse_2(&self, v: u8) -> u8 {
        if self.pulse_2_mute || (!self.pulse_2_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }

    fn triangle(&self, v: u8) -> u8 {
        if self.triangle_mute || (!self.triangle_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }

    fn noise(&self, v: u8) -> u8 {
        if self.noise_mute || (!self.noise_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }

    fn dmc(&self, v: u8) -> u8 {
        if self.dmc_mute || (!self.dmc_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }

    fn ext(&self, v: i16) -> i16 {
        if self.ext_mute || (!self.ext_solo && self.any_solo()) {
            0
        } else {
            v
        }
    }
}

pub trait Sample: Copy + Default {
    type Mixer: SampleMixer<Self>;
}

pub trait SampleMixer<S>: Default {
    fn mix(&self, pulse_1: u8, pulse_2: u8, triangle: u8, noise: u8, dmc: u8, ext: i16) -> S;
}

impl Sample for i16 {
    type Mixer = I16LutMixer;
}

pub struct I16LutMixer {
    pulse_table: Vec<i16>,
    tnd_table: Vec<i16>,
}

impl Default for I16LutMixer {
    fn default() -> Self {
        let mut pulse_table = Vec::new();
        pulse_table.push(0);
        for x in 1..32 {
            let x = x as f64;
            let f_val = 95.52 / (8128.0 / x + 100.0);
            let val = (f_val.clamp(0.0, 1.0) * i16::MAX as f64).round();
            pulse_table.push(val as i16);
        }

        let mut tnd_table = Vec::new();
        tnd_table.push(0);
        for x in 1..204 {
            let x = x as f64;
            let f_val = 163.67 / (24329.0 / x + 100.0);
            let val = (f_val.clamp(0.0, 1.0) * i16::MAX as f64).round();
            tnd_table.push(val as i16);
        }

        Self {
            pulse_table,
            tnd_table,
        }
    }
}

impl SampleMixer<i16> for I16LutMixer {
    fn mix(&self, pulse_1: u8, pulse_2: u8, triangle: u8, noise: u8, dmc: u8, ext: i16) -> i16 {
        let pulse_out = self.pulse_table[(pulse_1 + pulse_2) as usize];
        let tnd_out = self.tnd_table[(3 * triangle + 2 * noise + dmc) as usize];
        ext - (pulse_out + tnd_out)
    }
}

impl Sample for f32 {
    type Mixer = ();
}

impl SampleMixer<f32> for () {
    fn mix(&self, pulse_1: u8, pulse_2: u8, triangle: u8, noise: u8, dmc: u8, ext: i16) -> f32 {
        let pulse_out = if pulse_1 + pulse_2 == 0 {
            0.0
        } else {
            let pulse_1 = pulse_1 as f32;
            let pulse_2 = pulse_2 as f32;
            95.88 / ((8128.0 / (pulse_1 + pulse_2)) + 100.0)
        };

        let tnd_out = if triangle + noise + dmc == 0 {
            0.0
        } else {
            let triangle = triangle as f32;
            let noise = noise as f32;
            let dmc = dmc as f32;
            159.79 / (1.0 / ((triangle / 8227.0) + (noise / 12241.0) + (dmc / 22638.0)) + 100.0)
        };

        let ext = ext as f32;
        let ext_out = ext / i16::MAX as f32;

        ext_out - (pulse_out + tnd_out)
    }
}
