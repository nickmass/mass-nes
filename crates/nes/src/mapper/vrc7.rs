use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::PpuFetchKind;

use super::vrc_irq::VrcIrq;
use super::{Nametable, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum Vrc7Variant {
    Undefined,
    Vrc7a,
    Vrc7b,
}

impl Vrc7Variant {
    fn register_decode(&self, addr: u16) -> u16 {
        if addr < 0x8000 {
            return addr;
        }
        let a3 = addr & 0x0008;
        let a4 = addr & 0x0010;
        let addr = addr & 0xffe7;

        match self {
            Vrc7Variant::Undefined => {
                if a3 != 0 {
                    addr | (a3 << 1) | (a4 >> 1)
                } else {
                    addr | a3 | a4
                }
            }
            Vrc7Variant::Vrc7a => addr | a3 | a4,
            Vrc7Variant::Vrc7b => addr | (a3 << 1) | (a4 >> 1),
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Vrc7 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    mirroring: SimpleMirroring,
    audio: Audio,
    variant: Vrc7Variant,
    #[cfg_attr(feature = "save-states", save(nested))]
    irq: VrcIrq,
    prg_ram: FixedMemoryBlock<8>,
    chr_ram: Option<FixedMemoryBlock<8>>,
    ram_protect: bool,
    prg_bank_regs: [u8; 4],
    chr_bank_regs: [u8; 8],
}

impl Vrc7 {
    pub fn new(mut cartridge: INes, variant: Vrc7Variant, debug: Rc<Debug>) -> Self {
        let mut prg_ram = FixedMemoryBlock::new();
        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }
        let last_bank = ((cartridge.prg_rom.len() / 0x2000) - 1) as u8;

        let chr_ram = (cartridge.chr_ram_bytes > 0).then(|| FixedMemoryBlock::new());

        let mirroring = SimpleMirroring::new(cartridge.mirroring);

        Self {
            cartridge,
            mirroring,
            audio: Audio::new(),
            variant,
            irq: VrcIrq::new(debug),
            prg_ram,
            chr_ram,
            ram_protect: false,
            prg_bank_regs: [0, 0, 0, last_bank],
            chr_bank_regs: [0; 8],
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7fff => self.prg_ram.read(addr),
            _ => {
                let bank_idx = (addr as usize >> 13) & 3;
                let bank = self.prg_bank_regs[bank_idx] as usize;
                self.cartridge.prg_rom.read_mapped(bank, 8 * 1024, addr)
            }
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        let addr = self.variant.register_decode(addr);
        match addr {
            0x6000..=0x7fff if !self.ram_protect => self.prg_ram.write(addr, value),
            0x8000 => self.prg_bank_regs[0] = value,
            0x8010 => self.prg_bank_regs[1] = value,
            0x9000 => self.prg_bank_regs[2] = value,
            0xa000..=0xdfff => {
                let reg = match addr {
                    0xa000 => 0,
                    0xa010 => 1,
                    0xb000 => 2,
                    0xb010 => 3,
                    0xc000 => 4,
                    0xc010 => 5,
                    0xd000 => 6,
                    0xd010 => 7,
                    _ => return,
                };
                self.chr_bank_regs[reg] = value;
            }
            0xe000 => {
                match value & 0x3 {
                    0 => self.mirroring.vertical(),
                    1 => self.mirroring.horizontal(),
                    2 => self.mirroring.internal_b(),
                    3 => self.mirroring.internal_a(),
                    _ => unreachable!(),
                }
                self.ram_protect = value & 0x80 == 0;
                self.audio.write(addr, value);
            }
            0x9010 | 0x9030 => self.audio.write(addr, value),
            0xe010 => self.irq.latch(value),
            0xf000 => self.irq.control(value),
            0xf010 => self.irq.acknowledge(),
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        let bank_idx = addr as usize >> 10 & 7;
        let bank = self.chr_bank_regs[bank_idx] as usize;

        if let Some(ram) = self.chr_ram.as_ref() {
            ram.read_mapped(bank, 1024, addr)
        } else {
            self.cartridge.chr_rom.read_mapped(bank, 1024, addr)
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr & 0x2000 == 0 {
            let bank_idx = addr as usize >> 10 & 7;
            let bank = self.chr_bank_regs[bank_idx] as usize;

            if let Some(ram) = self.chr_ram.as_mut() {
                ram.write_mapped(bank, 1024, addr, value);
            }
        }
    }
}

impl Mapper for Vrc7 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.read_ppu(addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn tick(&mut self) {
        self.irq.tick();
        self.audio.tick();
    }

    fn get_sample(&self) -> Option<i16> {
        Some(self.audio.output())
    }

    fn get_irq(&self) -> bool {
        self.irq.irq()
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.save_wram()
        } else {
            None
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Audio {
    silence: bool,
    reg_select: u8,
    #[cfg_attr(feature = "save-states", serde(with = "serde_arrays"))]
    patches: [u8; 128],
    am_unit: AmUnit,
    fm_unit: FmUnit,
    channels: [Channel; 6],
    tick: u64,
    output: [i32; 6],
}

impl Audio {
    fn new() -> Self {
        let patches = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Custom
            0x03, 0x21, 0x05, 0x06, 0xE8, 0x81, 0x42, 0x27, // Buzzy Bell
            0x13, 0x41, 0x14, 0x0D, 0xD8, 0xF6, 0x23, 0x12, // Guitar
            0x11, 0x11, 0x08, 0x08, 0xFA, 0xB2, 0x20, 0x12, // Wurly
            0x31, 0x61, 0x0C, 0x07, 0xA8, 0x64, 0x61, 0x27, // Flute
            0x32, 0x21, 0x1E, 0x06, 0xE1, 0x76, 0x01, 0x28, // Clarinet
            0x02, 0x01, 0x06, 0x00, 0xA3, 0xE2, 0xF4, 0xF4, // Synth
            0x21, 0x61, 0x1D, 0x07, 0x82, 0x81, 0x11, 0x07, // Trumpet
            0x23, 0x21, 0x22, 0x17, 0xA2, 0x72, 0x01, 0x17, // Organ
            0x35, 0x11, 0x25, 0x00, 0x40, 0x73, 0x72, 0x01, // Bells
            0xB5, 0x01, 0x0F, 0x0F, 0xA8, 0xA5, 0x51, 0x02, // Vibes
            0x17, 0xC1, 0x24, 0x07, 0xF8, 0xF8, 0x22, 0x12, // Vibraphone
            0x71, 0x23, 0x11, 0x06, 0x65, 0x74, 0x18, 0x16, // Tutti
            0x01, 0x02, 0xD3, 0x05, 0xC9, 0x95, 0x03, 0x02, // Fretless
            0x61, 0x63, 0x0C, 0x00, 0x94, 0xC0, 0x33, 0xF6, // Synth Bass
            0x21, 0x72, 0x0D, 0x00, 0xC1, 0xD5, 0x56, 0x06, // Sweep
        ];
        Audio {
            silence: true,
            reg_select: 0,
            patches,
            am_unit: AmUnit::new(),
            fm_unit: FmUnit::new(),
            channels: Default::default(),
            tick: 0,
            output: [0; 6],
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xe000 => {
                self.silence = value & 0x40 != 0;
                if self.silence {
                    self.reg_select = 0;
                    for i in 0..8 {
                        self.patches[i] = 0;
                    }

                    for i in 0..6 {
                        self.channels[i].reset(&self.patches);
                    }
                }
            }
            0x9010 if !self.silence => self.reg_select = value,
            0x9030 if !self.silence => match self.reg_select {
                0x00..=0x07 => self.patches[self.reg_select as usize] = value,
                0x10..=0x15 => {
                    let chan = (self.reg_select & 0x0f) as usize;
                    self.channels[chan].write(&self.patches, 0, value);
                }
                0x20..=0x25 => {
                    let chan = (self.reg_select & 0x0f) as usize;
                    self.channels[chan].write(&self.patches, 1, value);
                }
                0x30..=0x35 => {
                    let chan = (self.reg_select & 0x0f) as usize;
                    self.channels[chan].write(&self.patches, 2, value);
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn tick(&mut self) {
        self.tick += 1;

        if self.silence {
            return;
        }

        let cycle = self.tick % 36;
        if cycle == 0 {
            self.am_unit.tick();
            self.fm_unit.tick();
        }

        if cycle % 6 == 0 {
            let channel = (cycle / 6) as usize;
            self.output[channel] =
                self.channels[channel].tick(self.am_unit.output, self.fm_unit.output);
        }
    }

    fn output(&self) -> i16 {
        if self.silence {
            return i16::MAX / 2;
        }

        let mut output = 0.0;
        for out in &self.output {
            output += *out as f32;
        }

        let out = ((output / 6.0) as i32 >> 5) + (i16::MAX as i32 / 2);
        out.min(i16::MAX as i32).max(0) as i16
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum SlotKind {
    Modulator,
    Carrier,
}

impl SlotKind {
    fn ctrl_reg(&self) -> usize {
        match self {
            SlotKind::Modulator => 0,
            SlotKind::Carrier => 1,
        }
    }

    fn ksl_reg(&self) -> usize {
        match self {
            SlotKind::Modulator => 2,
            SlotKind::Carrier => 3,
        }
    }

    fn ad_reg(&self) -> usize {
        match self {
            SlotKind::Modulator => 4,
            SlotKind::Carrier => 5,
        }
    }

    fn sr_reg(&self) -> usize {
        match self {
            SlotKind::Modulator => 6,
            SlotKind::Carrier => 7,
        }
    }

    fn half_sin_mask(&self) -> u8 {
        match self {
            SlotKind::Modulator => 0x08,
            SlotKind::Carrier => 0x10,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Instrument {
    regs: [u8; 8],
    kind: SlotKind,
}

impl Instrument {
    fn new(regs: &[u8], kind: SlotKind) -> Self {
        let regs = regs[0..8].try_into().unwrap();
        Self { regs, kind }
    }

    fn am(&self) -> bool {
        self.regs[self.kind.ctrl_reg()] & 0x80 != 0
    }

    fn fm(&self) -> bool {
        self.regs[self.kind.ctrl_reg()] & 0x40 != 0
    }

    fn percussion(&self) -> bool {
        self.regs[self.kind.ctrl_reg()] & 0x20 == 0
    }

    fn ksr(&self) -> bool {
        self.regs[self.kind.ctrl_reg()] & 0x10 != 0
    }

    fn multi(&self) -> u8 {
        const MULTI_LUT: [u8; 16] = [1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20, 24, 24, 30, 30];

        let multi = self.regs[self.kind.ctrl_reg()] & 0x0f;
        MULTI_LUT[multi as usize]
    }

    fn ksl(&self) -> u8 {
        self.regs[self.kind.ksl_reg()] >> 6
    }

    fn base_attenuation(&self) -> u8 {
        self.regs[2] & 0x3f
    }

    fn half_sin(&self) -> bool {
        self.regs[3] & self.kind.half_sin_mask() != 0
    }

    fn feedback_level(&self) -> u8 {
        self.regs[3] & 0x7
    }

    fn attack_rate(&self) -> u8 {
        self.regs[self.kind.ad_reg()] >> 4
    }

    fn decay_rate(&self) -> u8 {
        self.regs[self.kind.ad_reg()] & 0xf
    }

    fn sustain_level(&self) -> u8 {
        self.regs[self.kind.sr_reg()] >> 4
    }

    fn release_rate(&self) -> u8 {
        self.regs[self.kind.sr_reg()] & 0xf
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Channel {
    carrier: Slot,
    modulator: Slot,
    regs: [u8; 3],
    mod_output: i32,
}

impl Channel {
    fn frequency(&self) -> u16 {
        self.regs[0] as u16 | (((self.regs[1] & 0x01) as u16) << 8)
    }

    fn octave(&self) -> u8 {
        (self.regs[1] >> 1) & 0x07
    }

    fn key_on(&self) -> bool {
        self.regs[1] & 0x10 != 0
    }

    fn sustain_on(&self) -> bool {
        self.regs[1] & 0x20 != 0
    }

    fn volume(&self) -> u8 {
        self.regs[2] & 0x0f
    }

    fn instrument(&self) -> u8 {
        self.regs[2] >> 4
    }

    fn write(&mut self, patches: &[u8], reg: u8, value: u8) {
        match reg {
            0 => self.regs[0] = value,
            1 => {
                let was_key_on = self.key_on();
                self.regs[1] = value;
                if was_key_on != self.key_on() {
                    if self.key_on() {
                        let inst = &patches[self.instrument() as usize * 8..];
                        self.carrier.inst = Instrument::new(inst, SlotKind::Carrier);
                        self.modulator.inst = Instrument::new(inst, SlotKind::Modulator);
                        self.carrier.key_on();
                        self.modulator.key_on();
                    } else {
                        self.carrier.key_off();
                        self.modulator.key_off();
                    }
                }
            }
            2 => self.regs[2] = value,
            _ => {}
        }
    }

    fn tick(&mut self, am_out: f32, fm_out: f32) -> i32 {
        self.mod_output = self.modulator.tick(
            self.frequency() as u32,
            self.octave() as u32,
            self.volume(),
            self.sustain_on(),
            self.mod_output,
            fm_out,
            am_out,
        );

        self.carrier.tick(
            self.frequency() as u32,
            self.octave() as u32,
            self.volume(),
            self.sustain_on(),
            self.mod_output,
            fm_out,
            am_out,
        )
    }

    fn reset(&mut self, patches: &[u8]) {
        self.write(patches, 0, 0);
        self.write(patches, 1, 0);
        self.write(patches, 2, 0);
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            carrier: Slot::new(SlotKind::Carrier, &[0; 8]),
            modulator: Slot::new(SlotKind::Modulator, &[0; 8]),
            regs: [0; 3],
            mod_output: 0,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum EnvelopePhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Slot {
    kind: SlotKind,
    inst: Instrument,
    phase: u32,
    prev_output: i32,
    envelope_phase: EnvelopePhase,
    egc: u32,
}

use std::f32::consts::PI;

use std::sync::LazyLock;
const SIN_LUT_BIT_WIDTH: usize = 12;
const SIN_LUT_LEN: usize = 1 << SIN_LUT_BIT_WIDTH;

static HALF_SIN_TABLE: LazyLock<[f32; SIN_LUT_LEN]> = LazyLock::new(|| {
    let mut table = [0.0; SIN_LUT_LEN];
    for i in 0..SIN_LUT_LEN {
        let scale = i as f32 / SIN_LUT_LEN as f32;
        table[i] = to_db((PI * scale).sin());
    }
    table
});

const MAX_DB: u32 = 1 << 23;

fn to_linear(db: f32) -> f32 {
    if db >= 48.0 {
        return 0.0;
    }
    10.0f32.powf(db / -20.0)
}

fn to_db(linear: f32) -> f32 {
    if linear == 0.0 {
        return f32::INFINITY;
    }

    -20.0 * linear.log10()
}

impl Slot {
    fn new(kind: SlotKind, inst: &[u8]) -> Self {
        Self {
            kind,
            inst: Instrument::new(inst, kind),
            phase: 0,
            prev_output: 0,
            envelope_phase: EnvelopePhase::Idle,
            egc: 0,
        }
    }

    fn tick(
        &mut self,
        freq: u32,
        octave: u32,
        volume: u8,
        sustain_on: bool,
        mod_out: i32,
        fm_out: f32,
        am_out: f32,
    ) -> i32 {
        let fm = if self.inst.fm() { fm_out } else { 1.0 };
        let phase_inc = (freq * (1 << octave) * self.inst.multi() as u32) as f32 * fm / 2.0;
        self.phase = self.phase.wrapping_add(phase_inc as u32);
        self.phase &= 0x3ffff;

        let adj = match self.kind {
            SlotKind::Modulator => match self.inst.feedback_level() {
                0 => 0,
                f => mod_out >> (9 - f),
            },
            SlotKind::Carrier => mod_out,
        };

        let phase_secondary = (self.phase as i32).wrapping_add(adj);
        let rectify = phase_secondary & 0x20000 != 0;
        let sin_index = (phase_secondary & 0x1ffff) >> (17 - SIN_LUT_BIT_WIDTH);

        let base = match self.kind {
            SlotKind::Modulator => 0.75 * self.inst.base_attenuation() as f32,
            SlotKind::Carrier => 3.0 * volume as f32,
        };

        let key_scale = match self.inst.ksl() {
            0 => 0.0,
            k => {
                const KEY_SCALE_LUT: [f32; 16] = [
                    0.00, 18.00, 24.00, 27.75, 30.00, 32.25, 33.75, 35.25, 36.00, 37.50, 38.25,
                    39.00, 39.75, 40.50, 41.25, 42.00,
                ];

                let f = (freq >> 5) as usize;
                let b = octave as f32;
                let a = KEY_SCALE_LUT[f] - 6.0 * (7.0 - b);

                if a <= 0.0 {
                    0.0
                } else if k == 3 {
                    a
                } else {
                    a / (2.0f32.powi(3 - k as i32))
                }
            }
        };
        let am = if self.inst.am() { am_out } else { 0.0 };

        let envelope = self.envelope(sustain_on, freq, octave) * 48.0;

        let total = HALF_SIN_TABLE[sin_index as usize] + base + key_scale + envelope + am;

        let linear = to_linear(total).min(1.0).max(0.0);

        let output = (linear * (1 << 20) as f32) as i32;

        let output = if rectify {
            if self.inst.half_sin() { 0 } else { -output }
        } else {
            output
        };

        let mixed = (output + self.prev_output) / 2;
        self.prev_output = output;
        mixed
    }

    fn envelope(&mut self, sustain_on: bool, freq: u32, octave: u32) -> f32 {
        let bf = (freq >> 8) + (octave << 1);
        let kb = if self.inst.ksr() { bf } else { bf >> 2 };

        let r = match self.envelope_phase {
            EnvelopePhase::Attack => self.inst.attack_rate(),
            EnvelopePhase::Decay => self.inst.decay_rate(),
            EnvelopePhase::Sustain if self.inst.percussion() => self.inst.release_rate(),
            EnvelopePhase::Sustain => 0,
            EnvelopePhase::Release if sustain_on => 5,
            EnvelopePhase::Release if self.inst.percussion() => self.inst.release_rate(),
            EnvelopePhase::Release => 7,
            EnvelopePhase::Idle => 0,
        };

        let r = r as u32;
        let rks = r * 4 + kb;
        let rh = rks >> 2;
        let rh = rh.min(15);
        let rl = rks & 3;

        let adj = match self.envelope_phase {
            _ if r == 0 => 0,
            EnvelopePhase::Attack => (12 * (rl + 4)) << rh,
            EnvelopePhase::Idle => 0,
            _ => (rl + 4) << (rh - 1),
        };

        self.egc += adj;

        match self.envelope_phase {
            EnvelopePhase::Attack => {
                if self.egc >= MAX_DB {
                    self.egc = 0;
                    self.envelope_phase = EnvelopePhase::Decay;
                    return 0.0;
                }

                return 1.0 - ((self.egc as f32).ln() / (MAX_DB as f32).ln());
            }
            EnvelopePhase::Decay => {
                let sustain = 3.0 * self.inst.sustain_level() as f32 * MAX_DB as f32 / 48.0;
                let sustain = sustain as u32;
                if self.egc >= sustain {
                    self.egc = sustain;
                    self.envelope_phase = EnvelopePhase::Sustain;
                }
            }
            EnvelopePhase::Sustain => {
                if self.egc >= MAX_DB {
                    self.egc = MAX_DB;
                    self.envelope_phase = EnvelopePhase::Idle;
                }
            }
            EnvelopePhase::Release => {
                if self.egc >= MAX_DB {
                    self.egc = MAX_DB;
                    self.envelope_phase = EnvelopePhase::Idle;
                }
            }
            EnvelopePhase::Idle => return 1.0,
        }

        self.egc as f32 / MAX_DB as f32
    }

    fn key_on(&mut self) {
        self.egc = 0;
        self.phase = 0;
        self.envelope_phase = EnvelopePhase::Attack;
    }

    fn key_off(&mut self) {
        if matches!(self.envelope_phase, EnvelopePhase::Attack) {
            let output = 1.0 - ((self.egc as f32).ln() / (MAX_DB as f32).ln());
            self.egc = (output * MAX_DB as f32) as u32;
        }
        self.envelope_phase = EnvelopePhase::Release;
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct AmUnit {
    counter: u32,
    output: f32,
}

impl AmUnit {
    fn new() -> Self {
        Self {
            counter: 0,
            output: 0.0,
        }
    }

    fn tick(&mut self) {
        self.counter += 78;
        self.counter &= 0xfffff;

        let counter = self.counter as f32 / (1 << 20) as f32;
        let sinx = (2.0 * PI * counter).sin();

        self.output = (1.0 + sinx) * 0.6;
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct FmUnit {
    counter: u32,
    output: f32,
}

impl FmUnit {
    fn new() -> Self {
        Self {
            counter: 0,
            output: 0.0,
        }
    }

    fn tick(&mut self) {
        self.counter += 105;
        self.counter &= 0xfffff;

        let counter = self.counter as f32 / (1 << 20) as f32;
        let sinx = (2.0 * PI * counter).sin();

        self.output = 2.0f32.powf(13.75 / 1200.0 * sinx);
    }
}
