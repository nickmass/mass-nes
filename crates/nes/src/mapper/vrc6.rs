#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::Cartridge;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};

use super::{Nametable, SimpleMirroring};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum VrcIrqMode {
    Cycle,
    Scanline,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct VrcIrq {
    counter: u8,
    scanline_counter: i16,
    latch: u8,
    mode: VrcIrqMode,
    enabled: bool,
    renable: bool,
    triggered: bool,
}

impl VrcIrq {
    pub fn new() -> Self {
        Self {
            counter: 0,
            scanline_counter: 341,
            latch: 0,
            mode: VrcIrqMode::Cycle,
            enabled: false,
            renable: false,
            triggered: false,
        }
    }

    pub fn tick(&mut self) {
        self.scanline_counter -= 3;
        match self.mode {
            VrcIrqMode::Cycle => {
                self.scanline_counter += 341;
                self.trigger();
            }
            VrcIrqMode::Scanline => {
                if self.scanline_counter <= 0 {
                    self.scanline_counter += 341;
                    self.trigger();
                }
            }
        }
    }

    fn trigger(&mut self) {
        if self.counter == 0xff {
            if self.enabled {
                self.triggered = true;
            }
            self.counter = self.latch;
            self.enabled = self.renable;
        } else {
            self.counter += 1;
        }
    }

    pub fn irq(&self) -> bool {
        self.triggered
    }

    pub fn latch(&mut self, value: u8) {
        self.latch = value;
    }

    pub fn control(&mut self, value: u8) {
        self.renable = value & 0x1 != 0;
        self.enabled = value & 0x2 != 0;
        self.mode = if value & 0x4 != 0 {
            VrcIrqMode::Cycle
        } else {
            VrcIrqMode::Scanline
        };

        self.triggered = false;
        self.scanline_counter = 341;

        if self.enabled {
            self.counter = self.latch;
        }
    }

    pub fn acknowledge(&mut self) {
        self.triggered = false;
    }
}

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
    cartridge: Cartridge,
    variant: Vrc6Variant,
    mirroring: SimpleMirroring,
    irq: VrcIrq,
    prg: MappedMemory,
    chr: MappedMemory,
    ram_protect: bool,
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
    pub fn new(cartridge: Cartridge, variant: Vrc6Variant) -> Self {
        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());
        let mut prg = MappedMemory::new(&cartridge, 0x6000, 8, 40, MemKind::Prg);
        let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);

        let last = (cartridge.prg_rom.len() / 0x2000) - 1;
        prg.map(0x6000, 8, 0, BankKind::Ram);
        prg.map(0x8000, 16, 0, BankKind::Rom);
        prg.map(0xc000, 8, 0, BankKind::Rom);
        prg.map(0xe000, 8, last, BankKind::Rom);

        let mix = (i16::MAX as f32 / 64.0) as i16;

        Self {
            cartridge,
            variant,
            mirroring,
            irq: VrcIrq::new(),
            prg,
            chr,
            ram_protect: false,
            chr_regs: [0; 8],
            chr_mode: 0,
            halt_audio: true,
            freq_mode: FreqMode::X1,
            pulse_a: Pulse::new(),
            pulse_b: Pulse::new(),
            sawtooth: Sawtooth::new(),
            mix,
        }
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        // todo: handle >= 0x2000 if nt chr rom mapped
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr < 0x8000 {
            if !self.ram_protect {
                self.prg.write(addr, value)
            }
            return;
        }

        let addr = self.variant.address(addr);
        match addr {
            0x8000..=0x8003 => {
                let bank = (value & 0xf) as usize;
                self.prg.map(0x8000, 16, bank, BankKind::Rom);
            }
            0xc000..=0xc003 => {
                let bank = (value & 0x1f) as usize;
                self.prg.map(0xc000, 8, bank, BankKind::Rom);
            }
            0xb003 => {
                self.ram_protect = value & 0x80 != 0;
                self.chr_mode = value & 0x3f;
                self.sync_chr();
            }
            0xd000..=0xe003 => {
                let reg = addr & 0x3 | ((addr & 0x2000) >> 11);
                self.chr_regs[reg as usize] = value;
                self.sync_chr();
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
            0xb000 => self.sawtooth.accumulator_rate = value & 0x3f,
            0xb001 => self.sawtooth.freq_low(value),
            0xb002 => self.sawtooth.freq_high(value),
            _ => unreachable!(),
        }
    }

    fn sync_chr(&mut self) {
        let r = self.chr_regs;
        match self.chr_mode & 0x3 {
            0x0 => {
                self.chr.map(0x0000, 1, r[0] as usize, BankKind::Rom);
                self.chr.map(0x0400, 1, r[1] as usize, BankKind::Rom);
                self.chr.map(0x0800, 1, r[2] as usize, BankKind::Rom);
                self.chr.map(0x0c00, 1, r[3] as usize, BankKind::Rom);
                self.chr.map(0x1000, 1, r[4] as usize, BankKind::Rom);
                self.chr.map(0x1400, 1, r[5] as usize, BankKind::Rom);
                self.chr.map(0x1800, 1, r[6] as usize, BankKind::Rom);
                self.chr.map(0x1c00, 1, r[7] as usize, BankKind::Rom);
            }
            0x1 => {
                self.chr.map(0x0000, 2, r[0] as usize, BankKind::Rom);
                self.chr.map(0x0800, 2, r[1] as usize, BankKind::Rom);
                self.chr.map(0x1000, 2, r[2] as usize, BankKind::Rom);
                self.chr.map(0x1800, 2, r[3] as usize, BankKind::Rom);
            }
            0x2 | 0x3 => {
                self.chr.map(0x0000, 1, r[0] as usize, BankKind::Rom);
                self.chr.map(0x0400, 1, r[1] as usize, BankKind::Rom);
                self.chr.map(0x0800, 1, r[2] as usize, BankKind::Rom);
                self.chr.map(0x0c00, 1, r[3] as usize, BankKind::Rom);
                self.chr.map(0x1000, 2, r[4] as usize, BankKind::Rom);
                self.chr.map(0x1800, 2, r[5] as usize, BankKind::Rom);
            }
            _ => unreachable!(),
        }

        // todo: implement unused banking modes
        if self.chr_mode & 0x10 != 0 || self.chr_mode & 0x20 == 0 {
            unimplemented!()
        } else {
            let mirror_mode = self.chr_mode >> 2 & 0x3;
            match (self.chr_mode & 0x3, mirror_mode) {
                (0x0, 0x0) => self.mirroring.vertical(),
                (0x0, 0x1) => self.mirroring.horizontal(),
                (0x0, 0x2) => self.mirroring.internal_b(),
                (0x0, 0x3) => self.mirroring.internal_a(),
                _ => unimplemented!(),
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

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
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

    fn peek_ppu_fetch(&self, address: u16) -> Nametable {
        self.mirroring.ppu_fetch(address)
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
        let val =
            (self.pulse_a.sample as i16 + self.pulse_b.sample as i16 + self.sawtooth.sample as i16)
                * self.mix;
        Some(val)
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum FreqMode {
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

        if period == 0 {
            1
        } else {
            period
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Pulse {
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
    fn new() -> Self {
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

    fn tick(&mut self, freq: FreqMode) {
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

    fn volume(&mut self, value: u8) {
        self.volume = value & 0xf;
        self.duty = (value >> 4) & 0x7;
        self.constant = value & 0x80 != 0;
    }

    fn freq_low(&mut self, value: u8) {
        let period = (self.period & 0xff00) | value as u16;
        self.period = period;
    }

    fn freq_high(&mut self, value: u8) {
        let period = (self.period & 0xff) | ((value as u16 & 0xf) << 8);
        self.period = period;
        self.enabled = value & 0x80 != 0;
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Sawtooth {
    period: u16,
    counter: u16,
    accumulator_rate: u8,
    accumulator_counter: u8,
    accumulator: u8,
    enabled: bool,
    sample: u8,
}

impl Sawtooth {
    fn new() -> Self {
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

    fn tick(&mut self, freq: FreqMode) {
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

    fn freq_low(&mut self, value: u8) {
        let period = (self.period & 0xff00) | value as u16;
        self.period = period;
    }

    fn freq_high(&mut self, value: u8) {
        let period = (self.period & 0xff) | ((value as u16 & 0xf) << 8);
        self.period = period;
        self.enabled = value & 0x80 != 0;
    }
}
