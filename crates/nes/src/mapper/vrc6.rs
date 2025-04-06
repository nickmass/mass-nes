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

use super::Nametable;
use super::vrc_irq::VrcIrq;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum Vrc6Variant {
    A,
    B,
}

impl Vrc6Variant {
    fn address(&self, address: u16) -> u16 {
        match self {
            Vrc6Variant::A => address,
            Vrc6Variant::B => {
                let b0 = (address & 0x1) << 1;
                let b1 = (address & 0x2) >> 1;

                (address & 0xfffc) | b0 | b1
            }
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Vrc6 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    variant: Vrc6Variant,
    #[cfg_attr(feature = "save-states", save(nested))]
    irq: VrcIrq,
    prg_ram: FixedMemoryBlock<8>,
    ram_protect: bool,
    prg_regs: [u8; 3],
    chr_regs: [u8; 8],
    chr_mode: u8,

    halt_audio: bool,
    freq_mode: FreqMode,
    pulse_a: Pulse,
    pulse_b: Pulse,
    sawtooth: Sawtooth,
    mix: i16,
}

impl Vrc6 {
    pub fn new(mut cartridge: INes, variant: Vrc6Variant, debug: Rc<Debug>) -> Self {
        let mut prg_ram = FixedMemoryBlock::new();
        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }
        let last_bank = ((cartridge.prg_rom.len() / 0x2000) - 1) as u8;

        let mix = (i16::MAX as f32 / 64.0) as i16;

        Self {
            cartridge,
            variant,
            irq: VrcIrq::new(debug),
            prg_ram,
            ram_protect: true,
            prg_regs: [0, 0, last_bank],
            chr_regs: [0; 8],
            chr_mode: 0x20,
            halt_audio: true,
            freq_mode: FreqMode::X1,
            pulse_a: Pulse::new(),
            pulse_b: Pulse::new(),
            sawtooth: Sawtooth::new(),
            mix,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        if addr < 0x8000 {
            if self.ram_protect {
                0
            } else {
                self.prg_ram.read(addr)
            }
        } else {
            let (bank_idx, size) = match addr & 0xe000 {
                0x8000 | 0xa000 => (0, 16),
                0xc000 => (1, 8),
                0xe000 => (2, 8),
                _ => unreachable!(),
            };

            let bank = self.prg_regs[bank_idx] as usize;
            self.cartridge.prg_rom.read_mapped(bank, size * 1024, addr)
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr < 0x8000 {
            if !self.ram_protect {
                self.prg_ram.write(addr, value)
            }
            return;
        }

        let addr = self.variant.address(addr);
        match addr {
            0x8000..=0x8003 => self.prg_regs[0] = value & 0xf,
            0xc000..=0xc003 => self.prg_regs[1] = value & 0x1f,
            0xb003 => {
                self.ram_protect = value & 0x80 == 0;
                self.chr_mode = value & 0x3f;
            }
            0xd000..=0xe003 => {
                let reg = addr & 0x3 | ((addr & 0x2000) >> 11);
                self.chr_regs[reg as usize] = value;
            }
            0xf000 => self.irq.latch(value),
            0xf001 => self.irq.control(value),
            0xf002 => self.irq.acknowledge(),
            0xf003 => (),
            0x9003 => {
                self.halt_audio = value & 1 != 0;
                if value & 4 != 0 {
                    self.freq_mode = FreqMode::X256;
                } else if value & 2 != 0 {
                    self.freq_mode = FreqMode::X4;
                } else {
                    self.freq_mode = FreqMode::X1;
                }
            }
            0x9000 => self.pulse_a.volume(value),
            0x9001 => self.pulse_a.freq_low(value),
            0x9002 => self.pulse_a.freq_high(value),
            0xa000 => self.pulse_b.volume(value),
            0xa001 => self.pulse_b.freq_low(value),
            0xa002 => self.pulse_b.freq_high(value),
            0xa003 => (),
            0xb000 => self.sawtooth.accumulator_rate(value),
            0xb001 => self.sawtooth.freq_low(value),
            0xb002 => self.sawtooth.freq_high(value),
            _ => unreachable!(),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 != 0 {
            self.read_nt(addr)
        } else {
            self.read_chr(addr)
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let r = self.chr_regs;
        let a = addr as usize;
        let (bank, size) = match (self.chr_mode & 0x3, self.chr_mode & 0x20 != 0) {
            (0x0, _) => (r[a >> 10 & 7], 1),
            (0x1, true) => (r[a >> 11 & 3] >> 1, 2),
            (0x2 | 0x3, true) => match a & 0x1000 {
                0x0000 => (r[a >> 10 & 7], 1),
                0x1000 => (r[(a >> 11 & 3) + 2] >> 1, 2),
                _ => unreachable!(),
            },
            (0x1, false) => (r[a >> 11 & 3], 1),
            (0x2 | 0x3, false) => match a & 0x1000 {
                0x0000 => (r[a >> 10 & 7], 1),
                0x1000 => (r[(a >> 11 & 3) + 2], 1),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        self.cartridge
            .chr_rom
            .read_mapped(bank as usize, size * 1024, addr as u16)
    }

    fn read_nt(&self, addr: u16) -> u8 {
        let bank = self.map_nt(addr);

        self.cartridge
            .chr_rom
            .read_mapped(bank as usize, 1024, addr as u16)
    }

    fn map_nt(&self, addr: u16) -> u8 {
        let nt = (addr as usize >> 10) & 3;
        let r = self.chr_regs;

        let horz = r[6 + (nt >> 1)];
        let vert = r[6 + (nt & 1)];
        let four_screen = r[nt | 4];

        // Every game sets this bit, the unset bit logic was intended for a different board that was not released
        if self.chr_mode & 0x20 != 0 {
            let mirror_mode = self.chr_mode >> 2 & 0x3;
            match (self.chr_mode & 0x3, mirror_mode) {
                (0x0, 0x0) | (0x3, 0x1) => horz & 0xfe | nt as u8 & 1,
                (0x0, 0x1) | (0x3, 0x0) => vert & 0xfe | (nt as u8) >> 1,
                (0x0, 0x2) | (0x3, 0x3) => horz & 0xfe,
                (0x0, 0x3) | (0x3, 0x2) => vert | 1,
                (0x1, _) => four_screen,
                (0x2, 0x0 | 0x2) => vert,
                (0x2, 0x1 | 0x3) => horz,
                _ => unreachable!(),
            }
        } else {
            match self.chr_mode & 0xf {
                0x0 | 0x6 | 0x7 | 0x8 | 0xe | 0xf => horz,
                0x1 | 0x5 | 0x9 | 0xd => four_screen,
                0x2 | 0x3 | 0x4 | 0xa | 0xb | 0xc => vert,
                _ => unreachable!(),
            }
        }
    }
}

impl Mapper for Vrc6 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xf003));
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
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> Nametable {
        if self.chr_mode & 0x10 == 0x0 {
            if address & 0x2000 == 0 {
                Nametable::External
            } else {
                let nt = self.map_nt(address);
                if nt & 1 == 0 {
                    Nametable::InternalB
                } else {
                    Nametable::InternalA
                }
            }
        } else {
            Nametable::External
        }
    }

    fn get_irq(&mut self) -> bool {
        self.irq.irq()
    }

    fn tick(&mut self) {
        self.irq.tick();
        if !self.halt_audio {
            self.pulse_a.tick(self.freq_mode);
            self.pulse_b.tick(self.freq_mode);
            self.sawtooth.tick(self.freq_mode);
        }
    }

    fn get_sample(&self) -> Option<i16> {
        let val = (self.pulse_a.sample() as i16
            + self.pulse_b.sample() as i16
            + self.sawtooth.sample() as i16)
            * self.mix;
        Some(val)
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
#[derive(Debug, Copy, Clone)]
pub enum FreqMode {
    X1,
    X4,
    X256,
}

impl FreqMode {
    fn set(&self, period: u16) -> u16 {
        let period = match self {
            FreqMode::X1 => period,
            FreqMode::X4 => period >> 4,
            FreqMode::X256 => period >> 8,
        };

        period.max(1)
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Pulse {
    period: u16,
    counter: u16,
    volume: u8,
    duty: u8,
    duty_counter: u8,
    constant: bool,
    enabled: bool,
    sample: u8,
}

impl Pulse {
    pub fn new() -> Self {
        Self {
            period: 1,
            counter: 0,
            volume: 0,
            duty: 0,
            duty_counter: 0,
            constant: false,
            enabled: false,
            sample: 0,
        }
    }

    pub fn tick(&mut self, freq: FreqMode) {
        if self.enabled && !self.constant {
            if self.counter == 0 {
                if self.duty_counter == 0 {
                    self.duty_counter = 15;
                } else {
                    self.duty_counter -= 1;
                    if self.duty_counter <= self.duty {
                        self.sample = self.volume
                    } else {
                        self.sample = 0;
                    }
                }
                self.counter = freq.set(self.period);
            } else {
                self.counter -= 1;
            }
        } else if !self.enabled {
            self.duty_counter = 15;
            self.sample = 0;
        } else if self.constant {
            self.sample = self.volume;
        }
    }

    pub fn volume(&mut self, value: u8) {
        self.volume = value & 0xf;
        self.duty = (value >> 4) & 0x7;
        self.constant = value & 0x80 != 0;
    }

    pub fn freq_low(&mut self, value: u8) {
        let period = (self.period & 0xff00) | value as u16;
        self.period = period;
    }

    pub fn freq_high(&mut self, value: u8) {
        let period = (self.period & 0xff) | ((value as u16 & 0xf) << 8);
        self.period = period;
        self.enabled = value & 0x80 != 0;
    }

    pub fn sample(&self) -> u8 {
        self.sample
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Sawtooth {
    period: u16,
    counter: u16,
    accumulator_rate: u8,
    accumulator_counter: u8,
    accumulator: u8,
    enabled: bool,
    sample: u8,
}

impl Sawtooth {
    pub fn new() -> Self {
        Self {
            period: 1,
            counter: 0,
            accumulator_rate: 0,
            accumulator_counter: 0,
            accumulator: 0,
            enabled: false,
            sample: 0,
        }
    }

    pub fn tick(&mut self, freq: FreqMode) {
        if self.enabled {
            if self.counter == 0 {
                self.counter = freq.set(self.period);
                self.accumulator_counter += 1;
                if self.accumulator_counter & 1 == 0 {
                    self.accumulator = self.accumulator.wrapping_add(self.accumulator_rate);
                    self.sample = self.accumulator >> 3;
                }

                if self.accumulator_counter == 14 {
                    self.accumulator = 0;
                    self.sample = 0;
                    self.accumulator_counter = 0;
                }
            } else {
                self.counter -= 1;
            }
        } else if !self.enabled {
            self.accumulator_counter = 0;
            self.accumulator = 0;
            self.sample = 0;
        }
    }

    pub fn accumulator_rate(&mut self, value: u8) {
        self.accumulator_rate = value & 0x3f;
    }

    pub fn freq_low(&mut self, value: u8) {
        let period = (self.period & 0xff00) | value as u16;
        self.period = period;
    }

    pub fn freq_high(&mut self, value: u8) {
        let period = (self.period & 0xff) | ((value as u16 & 0xf) << 8);
        self.period = period;
        self.enabled = value & 0x80 != 0;
    }

    pub fn sample(&self) -> u8 {
        self.sample
    }
}
