#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind};
use crate::ppu::PpuFetchKind;

use super::SimpleMirroring;

#[derive(Debug, Copy, Clone)]
enum Channel {
    A,
    B,
    C,
}

impl Channel {
    fn period_low_reg(&self) -> usize {
        match self {
            Channel::A => 0x0,
            Channel::B => 0x2,
            Channel::C => 0x4,
        }
    }

    fn period_high_reg(&self) -> usize {
        match self {
            Channel::A => 0x1,
            Channel::B => 0x3,
            Channel::C => 0x5,
        }
    }

    fn envelope_reg(&self) -> usize {
        match self {
            Channel::A => 0x8,
            Channel::B => 0x9,
            Channel::C => 0xa,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Fme7 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    prg: MappedMemory,
    chr: MappedMemory,
    chr_kind: BankKind,
    command: u8,
    parameter: u8,
    irq_enable: bool,
    irq_counter_enable: bool,
    irq_counter: u16,
    irq: bool,
    ram_protect: bool,
    ram_enable: bool,
    mirroring: SimpleMirroring,

    audio_enabled: bool,
    audio_protect: bool,
    audio_regs: [u8; 0x10],
    audio_reg_select: u8,
    audio_counter: u64,

    tone_counters: [u16; 3],
    noise_counter: u16,
    envelope_counter: u16,

    tone_state: [bool; 3],
    noise_seed: u32,
    envelope_volume: u8,
    envelope_ascending: bool,
    envelope_holding: bool,

    #[cfg_attr(feature = "save-states", save(skip))]
    audio_lookup: Vec<i16>,
    sample: i16,
}

impl Fme7 {
    pub fn new(mut cartridge: INes) -> Fme7 {
        let (chr, chr_kind) = if cartridge.chr_ram_bytes > 0 {
            let chr = MappedMemory::new(&cartridge, 0x0000, 8, 8, MemKind::Chr);
            (chr, BankKind::Ram)
        } else {
            let chr = MappedMemory::new(&cartridge, 0x0000, 0, 8, MemKind::Chr);
            (chr, BankKind::Rom)
        };
        let mut prg = MappedMemory::new(&cartridge, 0x6000, 16, 40, MemKind::Prg);
        prg.map(0x6000, 8, 0, BankKind::Ram);
        prg.map(
            0xe000,
            8,
            (cartridge.prg_rom.len() / 0x2000) - 1,
            BankKind::Rom,
        );

        if let Some(wram) = cartridge.wram.take() {
            prg.restore_wram(wram);
        }

        let mirroring = SimpleMirroring::new(cartridge.mirroring.into());

        let inc = 10.0f32.powf(1.0 / 10.0);
        let max = inc.powf(29.0);
        let sample_max = i16::MAX as f32;
        let channel_count = 3.0;

        let audio_lookup = (0..32)
            .map(|i| {
                if i < 2 {
                    return 0;
                }
                let factor = inc.powf(i as f32 - 2.0);
                let ratio = factor / max;
                (sample_max * ratio / channel_count) as i16
            })
            .collect();

        let mut mapper = Fme7 {
            cartridge,
            prg,
            chr,
            chr_kind,
            command: 0,
            parameter: 0,
            irq_enable: false,
            irq_counter_enable: false,
            irq_counter: 0,
            irq: false,
            ram_protect: false,
            ram_enable: false,
            mirroring,

            audio_enabled: false,
            audio_protect: false,
            audio_regs: [0; 0x10],
            audio_reg_select: 0,
            audio_counter: 0,

            tone_counters: [0; 3],
            noise_counter: 0,
            envelope_counter: 0,

            tone_state: [false; 3],
            noise_seed: 0xffff,
            envelope_volume: 7,
            envelope_ascending: true,
            envelope_holding: true,

            audio_lookup,
            sample: 0,
        };

        mapper.sync();

        mapper
    }

    fn read_cpu(&self, addr: u16) -> u8 {
        self.prg.read(&self.cartridge, addr)
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        self.chr.read(&self.cartridge, addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        if addr & 0xe000 == 0x6000 {
            if self.ram_enable && !self.ram_protect {
                self.prg.write(addr, value);
            }
            return;
        }

        match addr {
            0x8000 => {
                self.command = value & 0xf;
            }
            0xa000 => {
                self.parameter = value;
                self.sync();
            }
            0xc000 => {
                self.audio_protect = value & 0xf0 != 0;
                self.audio_reg_select = value & 0xf;
            }
            0xe000 => {
                if !self.audio_protect {
                    self.audio_enabled = true;
                    self.audio_regs[self.audio_reg_select as usize] = value;
                    if self.audio_reg_select == 0x0d {
                        self.envelope_counter = 0;
                        if self.envelope_attack() {
                            self.envelope_volume = 0;
                            self.envelope_ascending = true;
                        } else {
                            self.envelope_volume = 31;
                            self.envelope_ascending = false;
                        }
                        self.envelope_holding = false;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr < 0x2000 {
            self.chr.write(addr, value);
        }
    }

    fn sync(&mut self) {
        let bank = ((self.parameter & 0x3f) as usize) % self.cartridge.prg_rom.len();
        match self.command {
            0..=7 => self.chr.map(
                0x400 * self.command as u16,
                1,
                self.parameter as usize,
                self.chr_kind,
            ),
            8 => {
                self.ram_protect = self.parameter & 0x80 == 0;
                self.ram_enable = self.parameter & 0x40 != 0;
                if self.ram_enable {
                    self.prg.map(0x6000, 8, 0, BankKind::Ram);
                } else {
                    self.prg.map(0x6000, 8, bank, BankKind::Rom);
                }
            }
            9 => self.prg.map(0x8000, 8, bank, BankKind::Rom),
            0xa => self.prg.map(0xa000, 8, bank, BankKind::Rom),
            0xb => self.prg.map(0xc000, 8, bank, BankKind::Rom),
            0xc => match self.parameter & 0x3 {
                0 => self.mirroring.vertical(),
                1 => self.mirroring.horizontal(),
                2 => self.mirroring.internal_b(),
                3 => self.mirroring.internal_a(),
                _ => unreachable!(),
            },
            0xd => {
                self.irq_enable = self.parameter & 1 != 0;
                self.irq_counter_enable = self.parameter & 0x80 != 0;
                self.irq = false;
            }
            0xe => {
                self.irq_counter = (self.irq_counter & 0xff00) | self.parameter as u16;
            }
            0xf => {
                self.irq_counter = (self.irq_counter & 0x00ff) | ((self.parameter as u16) << 8);
            }
            _ => unreachable!(),
        }
    }

    fn tone_period(&self, channel: Channel) -> u16 {
        let low = self.audio_regs[channel.period_low_reg()] as u16;
        let high = self.audio_regs[channel.period_high_reg()] as u16 & 0xf;

        let period = high << 8 | low;
        period.max(1)
    }

    fn noise_period(&self) -> u16 {
        self.audio_regs[0x6] as u16 & 0x1f << 1
    }

    fn envelope_period(&self) -> u16 {
        ((self.audio_regs[0xc] as u16) << 8) | self.audio_regs[0xb] as u16
    }

    fn tone_enabled(&self, channel: Channel) -> bool {
        let val = self.audio_regs[0x7];

        match channel {
            Channel::A => val & 0x1 == 0,
            Channel::B => val & 0x2 == 0,
            Channel::C => val & 0x4 == 0,
        }
    }

    fn noise_enabled(&self, channel: Channel) -> bool {
        let val = self.audio_regs[0x7];

        match channel {
            Channel::A => val & 0x8 == 0,
            Channel::B => val & 0x10 == 0,
            Channel::C => val & 0x20 == 0,
        }
    }

    fn envelope_enabled(&self, channel: Channel) -> bool {
        self.audio_regs[channel.envelope_reg()] & 0x10 != 0
    }

    fn volume(&self, channel: Channel) -> u8 {
        let val = self.audio_regs[channel.envelope_reg()] & 0xf;
        if val == 0 { 0 } else { val * 2 + 1 }
    }

    fn tone(&self, channel: Channel) -> bool {
        match channel {
            Channel::A => self.tone_state[0],
            Channel::B => self.tone_state[1],
            Channel::C => self.tone_state[2],
        }
    }

    fn noise(&self) -> bool {
        self.noise_seed & 1 != 0
    }

    fn envelope_continue(&self) -> bool {
        self.audio_regs[0xd] & 0x08 != 0
    }

    fn envelope_attack(&self) -> bool {
        self.audio_regs[0xd] & 0x04 != 0
    }

    fn envelope_alternate(&self) -> bool {
        self.audio_regs[0xd] & 0x02 != 0
    }

    fn envelope_hold(&self) -> bool {
        self.audio_regs[0xd] & 0x01 != 0
    }

    fn envelope_at_limit(&self) -> bool {
        (self.envelope_ascending && self.envelope_volume == 31)
            || (!self.envelope_ascending && self.envelope_volume == 0)
    }

    fn channel_value(&self, channel: Channel) -> u8 {
        let active = (!self.tone_enabled(channel) || self.tone(channel))
            && (!self.noise_enabled(channel) || self.noise());
        if active {
            if self.envelope_enabled(channel) {
                self.envelope_volume
            } else {
                self.volume(channel)
            }
        } else {
            0
        }
    }

    fn audio_tick(&mut self) {
        self.audio_counter += 1;
        if self.audio_counter < 16 {
            return;
        }
        self.audio_counter = 0;

        self.noise_counter += 1;
        if self.noise_counter >= self.noise_period() {
            self.noise_counter = 0;
            if self.noise() {
                self.noise_seed ^= 0x24000;
            }
            self.noise_seed >>= 1;
        }

        self.envelope_counter += 1;
        if self.envelope_counter >= self.envelope_period() {
            self.envelope_counter = 0;
            if !self.envelope_holding {
                if self.envelope_at_limit() {
                    if !self.envelope_continue() {
                        self.envelope_holding = true;
                        self.envelope_volume = 0;
                    } else if self.envelope_hold() {
                        self.envelope_holding = true;
                        if self.envelope_alternate() {
                            self.envelope_volume ^= 0x1f;
                        }
                    } else if self.envelope_alternate() {
                        self.envelope_ascending = !self.envelope_ascending;
                    } else {
                        self.envelope_volume ^= 0x1f;
                    }
                }
            }

            if !self.envelope_holding {
                if self.envelope_ascending {
                    self.envelope_volume += 1;
                } else {
                    self.envelope_volume -= 1;
                }
            }
        }

        let channels = [Channel::A, Channel::B, Channel::C];
        let mut sample = 0;

        for (idx, channel) in channels.into_iter().enumerate() {
            self.tone_counters[idx] += 1;
            if self.tone_counters[idx] >= self.tone_period(channel) {
                self.tone_counters[idx] = 0;
                self.tone_state[idx] = !self.tone_state[idx];
            }

            sample += self.audio_lookup[self.channel_value(channel) as usize]
        }

        self.sample = sample;
    }
}

impl Mapper for Fme7 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xe000));
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

    fn tick(&mut self) {
        if self.irq_counter_enable {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            if self.irq_counter == 0xffff && self.irq_enable {
                self.irq = true;
            }
        }

        if self.audio_enabled {
            self.audio_tick();
        }
    }

    fn get_irq(&mut self) -> bool {
        self.irq
    }

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        self.mirroring.ppu_fetch(address)
    }

    fn get_sample(&self) -> Option<i16> {
        Some(self.sample)
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg.save_wram()
        } else {
            None
        }
    }
}
