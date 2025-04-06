#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::{
    bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, RangeAndMask},
    machine::{FdsInput, MapperInput},
    mapper::Mapper,
    memory::{Memory, MemoryBlock},
};

use super::SimpleMirroring;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DiskMode {
    Read,
    Write,
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Fds {
    #[cfg_attr(feature = "save-states", save(skip))]
    disk_sides: Vec<Vec<u8>>,
    #[cfg_attr(feature = "save-states", save(skip))]
    bios: Vec<u8>,
    prg_ram: MemoryBlock,
    chr_ram: MemoryBlock,
    mirroring: SimpleMirroring,
    timer_irq_counter: u16,
    timer_irq_reload_low: u8,
    timer_irq_reload_high: u8,
    timer_irq_repeat: bool,
    timer_irq_enabled: bool,
    timer_irq: bool,
    disk_irq_enabled: bool,
    disk_irq: bool,
    disk_motor_enabled: bool,
    enable_disk_io: bool,
    enable_sound_io: bool,
    disk_read_data: u8,
    disk_write_data: u8,
    disk_transfer_mode: DiskMode,
    disk_transfer_flag: bool,
    disk_index: usize,
    disk_transfer_counter: u64,
    disk_reset_transfer: bool,
    disk_ready: bool,
    disk_crc_ready: bool,
    disk_crc_control: bool,
    disk_prev_crc_control: bool,
    disk_gap_ended: bool,
    disk_side: Option<usize>,
    disk_swap_counter: u64,
    sound: Sound,
}

impl Fds {
    pub fn new(disk: crate::cartridge::Fds) -> Self {
        let prg_ram = MemoryBlock::new(32);
        let chr_ram = MemoryBlock::new(8);
        Fds {
            disk_sides: disk.disk_sides,
            bios: disk.bios,
            prg_ram,
            chr_ram,
            mirroring: SimpleMirroring::new(super::Mirroring::Vertical),
            timer_irq_counter: 0,
            timer_irq_reload_low: 0,
            timer_irq_reload_high: 0,
            timer_irq_repeat: false,
            timer_irq_enabled: false,
            timer_irq: false,
            disk_irq_enabled: false,
            disk_irq: false,
            disk_motor_enabled: false,
            enable_disk_io: true,
            enable_sound_io: true,
            disk_read_data: 0,
            disk_write_data: 0,
            disk_transfer_mode: DiskMode::Read,
            disk_transfer_flag: false,
            disk_index: 0,
            disk_transfer_counter: 0,
            disk_reset_transfer: false,
            disk_ready: false,
            disk_crc_ready: false,
            disk_crc_control: false,
            disk_prev_crc_control: false,
            disk_gap_ended: false,
            disk_side: Some(0),
            disk_swap_counter: 0,
            sound: Sound::new(),
        }
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
            addr if addr >= 0x6000 && addr < 0xe000 => {
                self.prg_ram.read_mapped(0, 32 * 1024, addr - 0x6000)
            }
            addr if addr >= 0xe000 => self.bios[addr as usize & 0x1fff],
            _ => 0,
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
        if addr >= 0x4040 && addr < 0x4098 {
            if self.enable_disk_io {
                return self.sound.read(addr);
            } else {
                return 0;
            }
        }

        match addr {
            0x4030 if self.enable_disk_io => {
                let mut value = 0;
                if self.timer_irq {
                    value |= 0x1;
                }
                if self.disk_transfer_flag {
                    value |= 0x2;
                }

                self.timer_irq = false;
                self.disk_irq = false;
                self.disk_transfer_flag = false;

                value
            } //disk status
            0x4031 if self.enable_disk_io => {
                self.disk_irq = false;
                self.disk_transfer_flag = false;
                self.disk_read_data
            } //read data
            0x4032 if self.enable_disk_io => {
                let mut value = 0;
                if self.disk_ejected() {
                    value |= 0x1;
                }
                if !self.disk_ready || self.disk_ejected() {
                    value |= 0x2;
                }
                if self.disk_ejected() {
                    value |= 0x4;
                }

                // write protect
                //value |= 0x4;

                value
            } //drive status
            0x4033 if self.enable_disk_io => 0x80, //external
            addr if addr >= 0x6000 && addr < 0xe000 => {
                self.prg_ram.read_mapped(0, 32 * 1024, addr - 0x6000)
            }
            addr if addr >= 0xe000 => self.bios[addr as usize & 0x1fff],
            _ => 0,
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr >= 0x4040 && addr < 0x4098 {
            if self.enable_disk_io {
                return self.sound.write(addr, value);
            } else {
                return;
            }
        }

        match addr {
            0x4020 => self.timer_irq_reload_low = value,  //timer low
            0x4021 => self.timer_irq_reload_high = value, //timer high
            0x4022 => {
                self.timer_irq_repeat = value & 0x1 != 0;
                self.timer_irq_enabled = value & 0x2 != 0 && self.enable_disk_io;
                if !self.timer_irq_enabled {
                    self.timer_irq = false;
                } else {
                    let lo = self.timer_irq_reload_low as u16;
                    let hi = (self.timer_irq_reload_high as u16) << 8;
                    self.timer_irq_counter = lo | hi;
                }
            } //irq ctl
            0x4023 => {
                self.enable_disk_io = value & 0x1 != 0;
                self.enable_sound_io = value & 0x2 != 0;

                if !self.enable_disk_io {
                    self.disk_irq = false;
                    self.timer_irq = false;
                    self.timer_irq_enabled = false;
                }
            } //master i/o
            0x4024 if self.enable_disk_io => {
                self.disk_irq = false;
                self.disk_transfer_flag = false;
                self.disk_write_data = value;
            } //write data
            0x4025 if self.enable_disk_io => {
                self.disk_irq = false;

                self.disk_motor_enabled = value & 0x01 != 0;
                self.disk_reset_transfer = value & 0x02 != 0;
                self.disk_transfer_mode = if value & 0x04 != 0 {
                    DiskMode::Read
                } else {
                    DiskMode::Write
                };
                if value & 0x08 != 0 {
                    self.mirroring.horizontal();
                } else {
                    self.mirroring.vertical();
                }
                self.disk_crc_control = value & 0x10 != 0;
                self.disk_crc_ready = value & 0x40 != 0;
                self.disk_irq_enabled = value & 0x80 != 0;
            } //fds ctl
            0x4026 if self.enable_disk_io => (),          //external
            addr if addr >= 0x6000 && addr < 0xe000 => {
                self.prg_ram
                    .write_mapped(0, 32 * 1024, addr - 0x6000, value)
            }
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr_ram.read_mapped(0, 8 * 1024, addr)
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr >= 0x2000 {
            return;
        }

        self.chr_ram.write_mapped(0, 8 * 1024, addr, value);
    }

    fn disk_read(&self) -> u8 {
        let Some(side) = self.disk_side else {
            return 0;
        };
        self.disk_sides[side][self.disk_index]
    }

    fn disk_write(&mut self, value: u8) {
        let Some(side) = self.disk_side else {
            return;
        };

        self.disk_sides[side][self.disk_index.saturating_sub(2)] = value;
    }

    fn disk_side_len(&self) -> usize {
        let Some(side) = self.disk_side else {
            return 0;
        };
        self.disk_sides[side].len()
    }

    fn disk_ejected(&self) -> bool {
        self.disk_side.is_none() || self.disk_swap_counter > 0
    }

    fn change_disk(&mut self, side: Option<usize>) {
        if let Some(side) = side {
            if side > self.disk_sides.len() {
                return;
            }

            self.disk_side = Some(side);
            self.disk_swap_counter = 2_000_000;
        } else {
            self.disk_side = None;
        }
        self.disk_index = 0;
        self.disk_motor_enabled = false;
        self.disk_ready = false;
    }
}

impl Mapper for Fds {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0xffff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_read(DeviceKind::Mapper, RangeAndMask(0x4020, 0x4100, 0xffff));
        cpu.register_write(DeviceKind::Mapper, RangeAndMask(0x4020, 0x4100, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.peek_cpu(addr),
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
            BusKind::Ppu => self.write_ppu(addr, value),
        }
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: crate::ppu::PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn tick(&mut self) {
        self.sound.tick();
        if self.timer_irq_enabled {
            if self.timer_irq_counter == 0 {
                self.timer_irq = true;
                let lo = self.timer_irq_reload_low as u16;
                let hi = (self.timer_irq_reload_high as u16) << 8;
                self.timer_irq_counter = lo | hi;
                if !self.timer_irq_repeat {
                    self.timer_irq_enabled = false;
                }
            } else {
                self.timer_irq_counter -= 1;
            }
        }

        if self.disk_swap_counter != 0 {
            self.disk_swap_counter -= 1;
        }

        if !self.disk_motor_enabled || self.disk_ejected() {
            self.disk_ready = false;
            self.disk_index = 0;
            self.disk_transfer_counter = 50000;
            self.disk_gap_ended = false;
            return;
        }

        if self.disk_reset_transfer && !self.disk_ready {
            return;
        }

        // based on Mesen, but missing much of the CRC logic
        if self.disk_transfer_counter != 0 {
            self.disk_transfer_counter -= 1;
        } else {
            self.disk_ready = true;
            self.disk_transfer_counter = 152;
            let mut need_irq = self.disk_irq_enabled;

            match self.disk_transfer_mode {
                DiskMode::Read => {
                    let disk_data = self.disk_read();

                    if !self.disk_crc_ready {
                        self.disk_gap_ended = false;
                    } else if !self.disk_gap_ended && disk_data != 0 {
                        self.disk_gap_ended = true;
                        need_irq = false;
                    }

                    if self.disk_gap_ended {
                        self.disk_read_data = disk_data;
                        self.disk_transfer_flag = true;
                        if need_irq {
                            self.disk_irq = true;
                        }
                    }
                }
                DiskMode::Write => {
                    let mut disk_data = 0;
                    if !self.disk_crc_control {
                        self.disk_transfer_flag = true;
                        disk_data = self.disk_write_data;
                        if need_irq {
                            self.disk_irq = true;
                        }
                    }

                    if !self.disk_crc_ready {
                        disk_data = 0;
                    }

                    if self.disk_crc_control {
                        disk_data = 0xff;
                    }

                    self.disk_write(disk_data);
                    self.disk_gap_ended = false;
                }
            };

            self.disk_prev_crc_control = self.disk_crc_control;

            self.disk_index += 1;
            if self.disk_index >= self.disk_side_len() {
                self.disk_motor_enabled = false;
                self.disk_index = 0;
            }
        }
    }

    fn get_irq(&mut self) -> bool {
        self.timer_irq | self.disk_irq
    }

    fn get_sample(&self) -> Option<i16> {
        Some(self.sound.output())
    }

    fn input(&mut self, input: MapperInput) {
        match input {
            MapperInput::Fds(fds) => match fds {
                FdsInput::SetDisk(side) => self.change_disk(side),
            },
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Sound {
    #[cfg_attr(feature = "save-states", serde(with = "serde_arrays"))]
    wavetable_ram: [u8; 64],
    wavetable_idx: usize,
    wavetable_hold: bool,
    wavetable_freq: u16,
    wavetable_timer: u16,
    wavetable_accumulator: u32,
    wavetable_gain: u8,
    wavetable_enable: bool,
    volume_envelope: Envelope,
    mod_envelope: Envelope,
    mod_table: [u8; 32],
    mod_timer: u8,
    mod_counter: i8,
    mod_freq: u16,
    mod_halt: bool,
    mod_carry: bool,
    mod_accumulator: u32,
    master_volume: Volume,
}

impl Sound {
    fn new() -> Self {
        Sound {
            wavetable_ram: [0; 64],
            wavetable_idx: 0,
            wavetable_hold: false,
            wavetable_freq: 0,
            wavetable_timer: 0,
            wavetable_accumulator: 0,
            wavetable_gain: 0,
            wavetable_enable: false,
            volume_envelope: Envelope::new(),
            mod_envelope: Envelope::new(),
            mod_table: [0; 32],
            mod_timer: 0,
            mod_counter: 0,
            mod_freq: 0,
            mod_halt: false,
            mod_carry: false,
            mod_accumulator: 0,
            master_volume: Volume::Full,
        }
    }
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            addr if addr >= 0x4040 && addr < 0x4080 && self.wavetable_hold => {
                self.wavetable_ram[(addr & 0x3f) as usize]
            }
            addr if addr >= 0x4040 && addr < 0x4080 => {
                self.wavetable_ram[self.wavetable_idx & 0x3f]
            }
            0x4090 => self.volume_envelope.gain() | 0x40,
            0x4091 => (self.wavetable_accumulator >> 12) as u8,
            0x4092 => self.mod_envelope.gain() | 0x40,
            0x4093 => (self.mod_accumulator >> 4 & 0x7f) as u8,
            0x4094 => {
                let temp = (self.mod_envelope.gain() as i16 * (self.mod_counter / 2) as i16) as u32;
                (temp >> 4) as u8
            }
            0x4095 => match self.mod_value() {
                0 => 0,
                1 => 1,
                2 => 2,
                3 => 4,
                4 => 0xc,
                5 => 0xc,
                6 => 0xe,
                7 => 0xf,
                _ => unreachable!(),
            },
            0x4096 => self.wavetable_ram[self.wavetable_idx & 0x3f] | 0x40,
            0x4097 => (self.mod_counter as u8) >> 1,
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            addr if addr >= 0x4040 && addr < 0x4080 && self.wavetable_hold => {
                self.wavetable_ram[(addr & 0x3f) as usize] = value & 0x3f;
            }
            0x4080 => self.volume_envelope.write_ctl(value),
            0x4082 => self.wavetable_freq = (self.wavetable_freq & 0xff00) | value as u16,
            0x4083 => {
                self.wavetable_freq = (self.wavetable_freq & 0x00ff) | ((value as u16) << 8);
                self.volume_envelope.write_freq_hi(value);
                self.mod_envelope.write_freq_hi(value);

                self.wavetable_enable = value & 0x80 == 0;
                if !self.wavetable_enable {
                    self.wavetable_accumulator = 0;
                    self.wavetable_timer = 0;
                    self.wavetable_idx = 0;
                }
            }
            0x4084 => {
                self.volume_envelope.write_ctl(value);
                if value & 0x80 != 0 && value & 0x3f == 0 {
                    self.wavetable_gain = 0;
                }
            }

            0x4085 => self.mod_counter = (value << 1) as i8,
            0x4086 => self.mod_freq = (self.mod_freq & 0xff00) | value as u16,
            0x4087 => {
                self.mod_freq = (self.mod_freq & 0x00ff) | ((value as u16) << 8);
                self.mod_carry = value & 0x40 != 0;
                self.mod_halt = value & 0x80 != 0;
                if self.mod_halt {
                    self.mod_accumulator &= 0xFF000;
                    self.mod_timer = 0;
                }
            }
            0x4088 if self.mod_halt => {
                self.mod_table[self.mod_address() as usize] = value & 0x7;
                self.mod_accumulator += 1 << 13;
            }
            0x4089 => {
                self.master_volume = match value & 3 {
                    0 => Volume::Full,
                    1 => Volume::TwoThirds,
                    2 => Volume::OneHalf,
                    3 => Volume::TwoFifths,
                    _ => unreachable!(),
                };

                self.wavetable_hold = value & 0x80 != 0;
            }
            0x408a => {
                self.volume_envelope.multiplier = value;
                self.mod_envelope.multiplier = value;
            }

            _ => (),
        }
    }

    fn mod_address(&self) -> u8 {
        (self.mod_accumulator >> 13) as u8 & 0x1f
    }

    fn mod_value(&self) -> u8 {
        self.mod_table[self.mod_address() as usize] & 0x7
    }

    fn wave_pitch(&self) -> u32 {
        let mut temp = (self.mod_envelope.gain() as i16 * (self.mod_counter / 2) as i16) as u32;
        if temp & 0x0f != 0 && temp & 0x800 == 0 {
            temp += 0x20;
        }

        temp += 0x400;
        temp = (temp >> 4) & 0xff;

        (temp * self.wavetable_freq as u32) & 0xfffff
    }

    fn tick(&mut self) {
        self.volume_envelope.tick();
        self.mod_envelope.tick();

        if self.mod_timer >= 16 {
            self.mod_timer = 0;
            let old_acc = self.mod_accumulator;
            self.mod_accumulator += self.mod_freq as u32;
            if self.mod_accumulator & 0x1000 != old_acc & 0x1000 {
                let inc = match self.mod_value() {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 4,
                    4 => {
                        self.mod_counter = 0;
                        0
                    }
                    5 => -4,
                    6 => -2,
                    7 => -1,
                    _ => unreachable!(),
                };

                self.mod_counter = self.mod_counter.wrapping_add(inc << 2);
            }
        } else if !self.mod_halt {
            self.mod_timer += 1;
        }

        if self.wavetable_timer >= 16 {
            self.wavetable_timer = 0;
            self.wavetable_accumulator += self.wave_pitch();
            self.wavetable_idx = ((self.wavetable_accumulator >> 18) & 0x3f) as usize;
        } else if self.wavetable_enable {
            self.wavetable_timer += 1;
        }

        if self.wavetable_idx == 0 {
            self.wavetable_gain = self.volume_envelope.gain().min(32);
        }
    }

    fn output(&self) -> i16 {
        let wave = self.wavetable_ram[self.wavetable_idx] as i16;
        let gain = self.wavetable_gain as i16;
        let volume = (wave * gain) << 3;

        self.master_volume.apply(volume)
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Volume {
    Full,
    TwoThirds,
    OneHalf,
    TwoFifths,
}

impl Volume {
    fn apply(&self, volume: i16) -> i16 {
        match self {
            Volume::Full => volume,
            Volume::TwoThirds => ((volume as i32 * 2) / 3) as i16,
            Volume::OneHalf => volume >> 1,
            Volume::TwoFifths => ((volume as i32 * 2) / 5) as i16,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Envelope {
    speed: u8,
    multiplier: u8,
    gain: u8,
    increase: bool,
    enabled: bool,
    counter: u64,
}

impl Envelope {
    fn new() -> Self {
        Envelope {
            speed: 0,
            multiplier: 0,
            gain: 0,
            increase: false,
            enabled: false,
            counter: 0,
        }
    }

    fn period(&self) -> u64 {
        8 * (self.speed as u64 + 1) * (self.multiplier as u64)
    }

    fn tick(&mut self) {
        if !self.enabled || self.multiplier == 0 {
            return;
        }

        if self.counter >= self.period() {
            self.reset();

            if self.increase && self.gain < 32 {
                self.gain += 1;
            } else if !self.increase && self.gain > 0 {
                self.gain -= 1;
            }
        } else {
            self.counter += 1;
        }
    }

    fn reset(&mut self) {
        self.counter = 0;
    }

    fn write_freq_hi(&mut self, value: u8) {
        self.reset();
        self.enabled = value & 0x40 == 0;
    }

    fn write_ctl(&mut self, value: u8) {
        self.reset();
        self.speed = value & 0x3f;
        if value & 0x80 != 0 {
            self.gain = self.speed;
        }
        self.increase = value & 0x40 != 0;
    }

    fn gain(&self) -> u8 {
        self.gain
    }
}
