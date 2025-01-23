#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::{
    bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, RangeAndMask},
    machine::{FdsInput, MapperInput},
    mapper::Mapper,
    memory::MemoryBlock,
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
    disk_gap_ended: bool,
    disk_side: Option<usize>,
    disk_swap_counter: u64,
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
            disk_gap_ended: false,
            disk_side: Some(0),
            disk_swap_counter: 0,
        }
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
            addr if addr >= 0x6000 && addr < 0xe000 => self.prg_ram.read(addr & 0x7fff),
            addr if addr >= 0xe000 => self.bios[addr as usize & 0x1fff],
            _ => 0,
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
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
                value |= 0x4;

                value
            } //drive status
            0x4033 if self.enable_disk_io => 0x80, //external
            addr if addr >= 0x6000 && addr < 0xe000 => self.prg_ram.read(addr & 0x7fff),
            addr if addr >= 0xe000 => self.bios[addr as usize & 0x1fff],
            _ => 0,
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr_ram.read(addr & 0x1fff)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
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
                self.disk_crc_ready = value & 0x40 != 0;
                self.disk_irq_enabled = value & 0x80 != 0;
            } //fds ctl
            0x4026 if self.enable_disk_io => (),          //external
            addr if addr >= 0x6000 && addr < 0xe000 => self.prg_ram.write(addr & 0x7fff, value),
            _ => (),
        }
    }

    fn write_ppu(&self, addr: u16, value: u8) {
        if addr >= 0x2000 {
            return;
        }

        self.chr_ram.write(addr, value);
    }

    fn disk_read(&self) -> u8 {
        let Some(side) = self.disk_side else {
            return 0;
        };
        self.disk_sides[side][self.disk_index]
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

        if self.disk_transfer_counter != 0 {
            self.disk_transfer_counter -= 1;
        } else {
            self.disk_ready = true;
            self.disk_transfer_counter = 152;

            let disk_data = match self.disk_transfer_mode {
                DiskMode::Read => self.disk_read(),
                DiskMode::Write => {
                    tracing::error!("FDS write currently unsupported");
                    self.disk_write_data
                }
            };

            let mut need_irq = self.disk_irq_enabled;

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

    fn input(&mut self, input: MapperInput) {
        match input {
            MapperInput::Fds(fds) => match fds {
                FdsInput::SetDisk(side) => self.change_disk(side),
            },
        }
    }
}
