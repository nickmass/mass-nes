#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use std::rc::Rc;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::INes;
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Namco163 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    prg: MappedMemory,
    chr: MappedMemory,
    prg_ram: bool,
    sound: Sound,
    irq_enabled: bool,
    irq_counter: u16,
    irq: bool,
    chr_bank_regs: [u8; 12],
    prg_bank_regs: [u8; 3],
    high_chr_ram: bool,
    low_chr_ram: bool,
    write_protect: [bool; 4],
}

impl Namco163 {
    pub fn new(mut cartridge: INes, debug: Rc<Debug>) -> Self {
        let prg_ram = cartridge.prg_ram_bytes > 0;
        let mut prg = if prg_ram {
            let mut prg = MappedMemory::new(&cartridge, 0x6000, 8, 40, MemKind::Prg);
            prg.map(0x6000, 8, 0, BankKind::Ram);

            if let Some(wram) = cartridge.wram.take() {
                prg.restore_wram(wram);
            }

            prg
        } else {
            MappedMemory::new(&cartridge, 0x8000, 0, 32, MemKind::Prg)
        };

        let last_bank = (cartridge.prg_rom.len() / 0x2000) - 1;
        prg.map(0xe000, 8, last_bank, BankKind::Rom);

        let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);

        let mut rom = Self {
            cartridge,
            debug,
            prg,
            chr,
            prg_ram,
            sound: Sound::new(),
            irq_enabled: false,
            irq_counter: 0,
            irq: false,
            chr_bank_regs: [0; 12],
            prg_bank_regs: [0; 3],
            low_chr_ram: false,
            high_chr_ram: false,
            write_protect: [true; 4],
        };

        rom.sync();
        rom
    }

    fn sync(&mut self) {
        for i in 0..3 {
            let bank = self.prg_bank_regs[i as usize] as usize;
            self.prg.map(0x8000 + i * 0x2000, 8, bank, BankKind::Rom);
        }

        for i in 0..8 {
            let bank = self.chr_bank_regs[i as usize] as usize;
            self.chr.map(i * 0x400, 1, bank, BankKind::Rom);
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
        match addr {
            0x4800..=0x4fff => self.sound.read(),
            0x5000..=0x57ff => (self.irq_counter & 0xff) as u8,
            0x5800..=0x5fff => (self.irq_counter >> 8) as u8,
            0x6000..=0x7fff if self.prg_ram => self.prg.read(&self.cartridge, addr),
            0x8000.. => self.prg.read(&self.cartridge, addr),
            _ => 0,
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x4800..=0x4fff => self.sound.write(value),
            0x5000..=0x57ff => {
                self.irq_counter = self.irq_counter & 0xff00 | value as u16;
                self.irq = false;
            }
            0x5800..=0x5fff => {
                self.irq_counter = (self.irq_counter & 0x00ff) | ((value as u16) << 8);
                self.irq_counter &= 0x7fff;
                self.irq_enabled = value & 0x80 != 0;
                self.irq = false;
            }
            0x8000..=0xdfff => {
                let reg = (addr - 0x8000) / 0x800;
                self.chr_bank_regs[reg as usize] = value;
                self.sync();
            }
            0xe000..=0xe7ff => {
                self.prg_bank_regs[0] = value & 0x3f;
                self.sound.enable(value);
                self.sync();
            }
            0xe800..=0xefff => {
                self.prg_bank_regs[1] = value & 0x3f;
                self.low_chr_ram = value & 0x40 == 0;
                self.high_chr_ram = value & 0x80 == 0;
                self.sync();
            }
            0xf000..=0xf7ff => {
                self.prg_bank_regs[2] = value & 0x3f;
                self.sync();
            }
            0xf800..=0xffff => {
                self.sound.address_port(value);

                if value & 0xf0 != 0x40 {
                    self.write_protect = [true; 4];
                } else {
                    self.write_protect[0] = value & 0x01 != 0;
                    self.write_protect[1] = value & 0x02 != 0;
                    self.write_protect[2] = value & 0x04 != 0;
                    self.write_protect[3] = value & 0x08 != 0;
                }
            }
            0x6000..=0x7fff if self.prg_ram => {
                let range = (addr - 0x6000) / 0x800;
                let write_protect = self.write_protect[range as usize];
                if !write_protect {
                    self.prg.write(addr, value)
                }
            }
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr < 0x2000 {
            self.chr.read(&self.cartridge, addr)
        } else {
            let addr = addr as usize;
            let bank = (addr & 0x2c00) / 0x400;
            let reg = self.chr_bank_regs[bank] as usize;
            self.cartridge
                .chr_rom
                .get(reg * 0x400 + (addr & 0x3ff))
                .copied()
                .unwrap_or_default()
        }
    }
}

impl Mapper for Namco163 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_read(DeviceKind::Mapper, RangeAndMask(0x4800, 0x6000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, RangeAndMask(0x4800, 0x6000, 0xffff));
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu if addr >= 0x6000 && self.prg_ram => self.prg.read(&self.cartridge, addr),
            BusKind::Cpu if addr >= 0x8000 => self.prg.read(&self.cartridge, addr),
            BusKind::Cpu => 0,
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

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        use super::Nametable;
        let (bank, chr_ram) = if address & 0x2000 != 0 {
            ((address & 0x2c00) / 0x400, true)
        } else {
            let bank = address / 0x400;
            let chr_ram = if bank < 4 {
                self.low_chr_ram
            } else {
                self.high_chr_ram
            };

            (bank, chr_ram)
        };

        let reg = self.chr_bank_regs[bank as usize];

        if reg >= 0xe0 && chr_ram {
            if reg & 1 == 0 {
                Nametable::InternalB
            } else {
                Nametable::InternalA
            }
        } else {
            Nametable::External
        }
    }

    fn tick(&mut self) {
        if self.irq_enabled && !self.irq {
            if self.irq_counter < 0x7fff {
                self.irq_counter += 1;
            } else {
                self.debug.event(crate::DebugEvent::MapperIrq);
                self.irq_counter = 0;
                self.irq = true;
            }
        }

        self.sound.tick();
    }

    fn get_irq(&mut self) -> bool {
        self.irq_enabled && self.irq
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery && self.prg_ram {
            self.prg.save_wram()
        } else {
            None
        }
    }

    fn get_sample(&self) -> Option<i16> {
        Some(self.sound.output())
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Sound {
    mem: Vec<u8>,
    addr: u8,
    increment: bool,
    enabled: bool,
    counter: u8,
    channel: u8,
    output: i16,
}

impl Sound {
    fn new() -> Self {
        Self {
            mem: vec![0; 128],
            addr: 0,
            increment: false,
            enabled: false,
            counter: 0,
            channel: 7,
            output: 0,
        }
    }

    fn read(&mut self) -> u8 {
        let value = self.mem[self.addr as usize];
        if self.increment {
            self.addr = self.addr.wrapping_add(1);
            self.addr &= 0x7f;
        }
        value
    }

    fn write(&mut self, value: u8) {
        self.mem[self.addr as usize] = value;
        if self.increment {
            self.addr = self.addr.wrapping_add(1);
            self.addr &= 0x7f;
        }
    }

    fn enable(&mut self, value: u8) {
        self.enabled = value & 0x40 == 0;
    }

    fn address_port(&mut self, value: u8) {
        self.addr = value & 0x7f;
        self.increment = value & 0x80 != 0;
    }

    fn enabled_channels(&self) -> u8 {
        ((self.mem[0x7f] >> 4) & 0x7) + 1
    }

    fn tick(&mut self) {
        if !self.enabled {
            return;
        }

        self.counter += 1;
        if self.counter < 15 {
            return;
        }

        self.counter = 0;
        let enabled_channels = self.enabled_channels();

        if 8 - self.channel > enabled_channels {
            self.channel = 7;
        }

        let mut channel = Channel::new(self.channel as usize, &mut self.mem);
        self.output = channel.tick();

        if self.channel == 0 {
            self.channel = 7;
        } else {
            self.channel -= 1;
        }
    }

    fn output(&self) -> i16 {
        self.output << 7
    }
}

struct Channel<'a> {
    mem: &'a mut [u8],
    reg_off: usize,
}

impl<'a> Channel<'a> {
    fn new(num: usize, mem: &'a mut [u8]) -> Self {
        let reg_off = num * 8 + 0x40;
        Self { mem, reg_off }
    }

    fn reg(&self, idx: usize) -> u8 {
        self.mem[self.reg_off + idx]
    }

    fn mut_reg(&mut self, idx: usize) -> &mut u8 {
        &mut self.mem[self.reg_off + idx]
    }

    fn length(&self) -> u32 {
        256 - (self.reg(4) & 0xfc) as u32
    }

    fn frequency(&self) -> u32 {
        let lo = self.reg(0) as u32;
        let mid = self.reg(2) as u32;
        let hi = (self.reg(4) & 3) as u32;

        lo | (mid << 8) | (hi << 16)
    }

    fn set_phase(&mut self, phase: u32) {
        let lo = (phase & 0xff) as u8;
        let mid = ((phase >> 8) & 0xff) as u8;
        let hi = ((phase >> 16) & 0xff) as u8;

        *self.mut_reg(1) = lo;
        *self.mut_reg(3) = mid;
        *self.mut_reg(5) = hi;
    }

    fn phase(&self) -> u32 {
        let lo = self.reg(1) as u32;
        let mid = self.reg(3) as u32;
        let hi = self.reg(5) as u32;

        lo | (mid << 8) | (hi << 16)
    }

    fn offset(&self) -> u32 {
        (self.reg(6) & 0xf) as u32
    }

    fn volume(&self) -> i16 {
        (self.reg(7) & 0xf) as i16
    }

    fn tick(&mut self) -> i16 {
        let freq = self.frequency();
        let phase = self.phase();
        let phase = (phase + freq) % (self.length() << 16);
        self.set_phase(phase);

        let sample_idx = ((phase >> 16) + self.offset()) & 0xff;
        let sample = (self.mem[sample_idx as usize / 2] >> ((sample_idx & 1) * 4)) & 0xf;
        let sample = sample as i16;

        (sample - 8) * self.volume()
    }
}
