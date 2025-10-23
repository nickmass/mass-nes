#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use super::fds::Sound as Fds;
use super::fme7::Sound as Sunsoft5b;
use super::mmc5::{Pcm, Pulse as Mmc5Pulse};
use super::namco163::Sound as N163;
use super::vrc6::{FreqMode, Pulse as Vrc6Pulse, Sawtooth};
use super::vrc7;
use crate::Region;
use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::NsfFile;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory, MemoryBlock, RomBlock};
use crate::ppu::PpuFetchKind;

static NSF_PLAYER_ROM: &[u8] = include_bytes!("nsf_player/nsf_player.bin");
static NSF_PLAYER_CHR: &[u8] = include_bytes!("nsf_player/ascii.chr");

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Nsf {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
    #[cfg_attr(feature = "save-states", save(skip))]
    file: NsfFile,
    banks: Option<[u8; 8]>,
    prg_ram: FixedMemoryBlock<8>,
    ex_ram: FixedMemoryBlock<1>,
    #[cfg_attr(feature = "save-states", save(skip))]
    sys_prg: RomBlock,
    #[cfg_attr(feature = "save-states", save(skip))]
    sys_chr: RomBlock,
    sys_ram: FixedMemoryBlock<1>,
    sys_nt_ram: FixedMemoryBlock<1>,
    fds_ram: Option<MemoryBlock>,
    fds_banks: Option<[u8; 2]>,
    play_timer_load: u32,
    play_timer: u32,
    play_pending: bool,
    current_song: u8,
    mul_left: u8,
    mul_right: u8,
    #[cfg_attr(feature = "save-states", save(nested))]
    sunsoft5b: Sunsoft5b,
    namco163: N163,
    vrc6: Vrc6,
    vrc7: vrc7::Audio,
    #[cfg_attr(feature = "save-states", save(nested))]
    mmc5: Mmc5,
    fds: Fds,
}

impl Nsf {
    pub fn new(region: Region, file: NsfFile) -> Nsf {
        if file.chips.vt02() {
            tracing::warn!("VT02 audio not supported in NSF");
        }

        if file.load_addr < 0x8000 && file.init_banks.is_none() {
            tracing::error!("Unexpected load_addr: {:04x}", file.load_addr);
        }

        let banks = file.init_banks;
        let prg_ram = FixedMemoryBlock::new();

        let mut fds_banks = None;
        let fds_ram = if file.chips.fds() {
            if let Some(banks) = banks {
                fds_banks = Some([banks[6], banks[7]]);
            }
            Some(MemoryBlock::new(32))
        } else {
            None
        };
        let ex_ram = FixedMemoryBlock::new();
        let sys_prg = RomBlock::new(NSF_PLAYER_ROM);
        let sys_chr = RomBlock::new(NSF_PLAYER_CHR);
        let sys_ram = FixedMemoryBlock::new();
        let sys_nt_ram = FixedMemoryBlock::new();

        let play_timer_load = match region {
            Region::Ntsc if file.ntsc_speed == 0 => 0.0,
            Region::Pal if file.pal_speed == 0 => 0.0,
            Region::Ntsc => region.cpu_clock() / (1000000.0 / file.ntsc_speed as f64),
            Region::Pal => region.cpu_clock() / (1000000.0 / file.pal_speed as f64),
        };

        let play_timer_load = play_timer_load as u32;
        let play_timer = 0;
        let play_pending = true;
        let current_song = file.starting_song.saturating_sub(1);

        let mut vrc7 = vrc7::Audio::new();
        vrc7.write(0xe000, 0x00);

        let mut namco163 = N163::new();
        namco163.enable(0x00);

        let mut fds = Fds::new();
        fds.write(0x4080, 0x80);
        fds.write(0x408a, 0xe8);

        Nsf {
            region,
            file,
            banks,
            prg_ram,
            ex_ram,
            sys_prg,
            sys_chr,
            sys_ram,
            sys_nt_ram,
            fds_banks,
            fds_ram,
            play_timer_load,
            play_timer,
            play_pending,
            mul_left: 0xff,
            mul_right: 0xff,
            sunsoft5b: Sunsoft5b::new(),
            current_song,
            namco163,
            vrc6: Vrc6::new(),
            vrc7,
            mmc5: Mmc5::new(),
            fds,
        }
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
            0x5010 | 0x5015 if self.file.chips.mmc5() => self.mmc5.read(addr),
            0x5205 if self.file.chips.mmc5() => {
                let val = self.mul_left as u16 * self.mul_right as u16;
                val as u8
            }
            0x5206 if self.file.chips.mmc5() => {
                let val = self.mul_left as u16 * self.mul_right as u16;
                (val >> 8) as u8
            }
            0x5300 => {
                if self.play_pending {
                    1
                } else {
                    0
                }
            }
            0x5301 => match self.region {
                Region::Ntsc => 0,
                Region::Pal => 1,
            },
            0x5303 => self.current_song,
            0x5304 => {
                let next = self.current_song.saturating_add(1);
                if next >= self.file.total_songs {
                    0
                } else {
                    next
                }
            }
            0x5305 => {
                if self.current_song == 0 {
                    self.file.total_songs.saturating_sub(1)
                } else {
                    self.current_song.saturating_sub(1)
                }
            }

            0x5310 => 0x20, // JSR
            0x5311 => (self.file.init_addr & 0xff) as u8,
            0x5312 => (self.file.init_addr >> 8) as u8,
            0x5313 => 0x60, // RTS

            0x5320 => 0x20, // JSR
            0x5321 => (self.file.play_addr & 0xff) as u8,
            0x5322 => (self.file.play_addr >> 8) as u8,
            0x5323 => 0x60, // RTS

            0x5400..=0x57ff => self.sys_prg.read_mapped(0, 1024, addr),
            0x5800..=0x5bff => self.sys_ram.read_mapped(0, 1024, addr),
            0x5c00..=0x5fff if self.file.chips.mmc5() => self.ex_ram.read_mapped(0, 1024, addr),
            0x6000..=0x7fff => self.read_prg_ram(addr),
            0xfffd | 0xfffb => 0x54,
            0xfffc => 0x03,
            0xfffa => 0x06,
            0x8000.. => self.read_prg(addr),
            _ => 0,
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
        match addr {
            0x4040..=0x4098 if self.file.chips.fds() => self.fds.read(addr),
            0x4800..=0x4fff if self.file.chips.namco163() => self.namco163.read(),
            0x5300 => {
                if self.play_pending {
                    self.play_pending = false;
                    1
                } else {
                    0
                }
            }
            0x5304 => {
                let next = self.current_song.saturating_add(1);
                let next = if next >= self.file.total_songs {
                    0
                } else {
                    next
                };
                self.current_song = next;
                self.current_song
            }
            0x5305 => {
                let prev = if self.current_song == 0 {
                    self.file.total_songs.saturating_sub(1)
                } else {
                    self.current_song.saturating_sub(1)
                };
                self.current_song = prev;
                self.current_song
            }
            0xfffd | 0xfffb => 0x54,
            0xfffc => 0x03,
            0xfffa => 0x06,
            0x8000.. => {
                let value = self.read_prg(addr);
                if self.file.chips.mmc5() {
                    self.mmc5.pcm.read(addr, value);
                }
                value
            }
            _ => self.peek_cpu(addr),
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x4040..=0x4098 if self.file.chips.fds() => self.fds.write(addr, value),
            0x4800..=0x4fff if self.file.chips.namco163() => {
                self.namco163.write(value);
            }
            0x5000..=0x5015 if self.file.chips.mmc5() => self.mmc5.write(addr, value),
            0x5205 if self.file.chips.mmc5() => self.mul_left = value,
            0x5206 if self.file.chips.mmc5() => self.mul_right = value,
            0x5302 => {
                self.banks = self.file.init_banks;

                if let Some((fds_banks, banks)) = self.fds_banks.as_mut().zip(self.banks.as_ref()) {
                    fds_banks[0] = banks[6];
                    fds_banks[1] = banks[7];
                }

                for a in 0..0x2000u16 {
                    self.prg_ram.write(a, 0x00);
                }

                text_line(&mut self.sys_nt_ram, 15, Some("Current Track:"));
                number_line(
                    &mut self.sys_nt_ram,
                    16,
                    Some(self.current_song.saturating_add(1)),
                );

                if let Some(fds_ram) = self.fds_ram.as_mut() {
                    let start = if self.banks.is_none() {
                        self.file.load_addr.saturating_sub(0x8000)
                    } else {
                        0x00
                    };
                    let end = (start as usize + self.file.data.len()).min(0xffff) as u16;
                    for a in 0..0x8000 {
                        let v = if a < start || a >= end {
                            0
                        } else {
                            let a = a - start;
                            self.file.data.read(a)
                        };
                        fds_ram.write(a, v);
                    }
                }
            }
            0x5800..=0x5bff => self.sys_ram.write_mapped(0, 1024, addr, value),
            0x5776 | 0x5777 if self.file.chips.fds() => {
                if let Some(banks) = self.fds_banks.as_mut() {
                    let bank_idx = (addr & 1) as usize;
                    banks[bank_idx] = value;
                }
            }
            0x5ff8..=0x5fff if self.banks.is_some() => {
                if let Some(banks) = self.banks.as_mut() {
                    let bank_idx = (addr & 7) as usize;
                    banks[bank_idx] = value;
                }
            }
            0x5c00..=0x5fff if self.file.chips.mmc5() => {
                self.ex_ram.write_mapped(0, 1024, addr, value)
            }
            0x9010 | 0x9030 if self.file.chips.vrc7() => self.vrc7.write(addr, value),
            0x9000..=0x9003 | 0xa000..=0xa002 | 0xb000..=0xb002 if self.file.chips.vrc6() => {
                self.vrc6.write(addr, value)
            }
            0xc000..=0xcfff if self.file.chips.sunsoft5b() => self.sunsoft5b.select(value),
            0xe000..=0xefff if self.file.chips.sunsoft5b() => self.sunsoft5b.value(value),
            0xf800..=0xffff if self.file.chips.namco163() => self.namco163.address_port(value),
            0x6000..=0x7fff => self.write_prg_ram(addr, value),
            _ => (),
        }

        if self.file.chips.fds() && addr >= 0x8000 {
            self.write_prg(addr, value);
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        if let Some(banks) = self.banks.as_ref() {
            let bank_idx = (addr & 0x7fff) >> 12;
            let bank = banks[bank_idx as usize];

            if let Some(fds_ram) = self.fds_ram.as_ref() {
                fds_ram.read_mapped(bank as usize, 4 * 1024, addr & 0x0fff)
            } else {
                self.file
                    .data
                    .read_mapped(bank as usize, 4 * 1024, addr & 0x0fff)
            }
        } else {
            if let Some(fds_ram) = self.fds_ram.as_ref() {
                fds_ram.read(addr)
            } else if addr < self.file.load_addr {
                2
            } else {
                let addr = addr - self.file.load_addr;

                if addr as usize >= self.file.data.len() {
                    return 2;
                }

                self.file.data.read(addr)
            }
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        let Some(fds_ram) = self.fds_ram.as_mut() else {
            return;
        };

        if let Some(banks) = self.banks.as_ref() {
            let bank_idx = (addr & 0x7fff) >> 12;
            let bank = banks[bank_idx as usize];
            fds_ram.write_mapped(bank as usize, 4 * 1024, addr & 0x0fff, value)
        } else {
            fds_ram.write(addr, value);
        }
    }

    fn read_prg_ram(&self, addr: u16) -> u8 {
        if let Some(bank) = self.fds_banks.as_ref()
            && self.file.chips.fds()
        {
            let bank_idx = match addr & 0xf000 {
                0x6000 => 0,
                0x7000 => 1,
                _ => return 0,
            };
            let bank = bank[bank_idx];
            if let Some(fds_ram) = self.fds_ram.as_ref() {
                fds_ram.read_mapped(bank as usize, 4 * 1024, addr & 0x0fff)
            } else {
                0
            }
        } else {
            self.prg_ram.read_mapped(0, 8 * 1024, addr)
        }
    }

    fn write_prg_ram(&mut self, addr: u16, value: u8) {
        if let Some(bank) = self.fds_banks.as_ref()
            && self.file.chips.fds()
        {
            let bank_idx = match addr & 0xf000 {
                0x6000 => 0,
                0x7000 => 1,
                _ => return,
            };
            let bank = bank[bank_idx];
            if let Some(fds_ram) = self.fds_ram.as_mut() {
                fds_ram.write_mapped(bank as usize, 4 * 1024, addr & 0x0fff, value)
            }
        } else {
            self.prg_ram.write_mapped(0, 8 * 1024, addr, value)
        }
    }

    fn peek_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 == 0 {
            self.sys_chr.read(addr.wrapping_sub(0x20 * 16) & 0xfff)
        } else {
            self.sys_nt_ram.read(addr & 0x3ff)
        }
    }
}

impl Mapper for Nsf {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));

        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));

        // Sys Reg
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x5300, 0x5fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x5300, 0x5fff));

        // Sys Rom
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5400, 0x5fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5400, 0x5fff));

        // Sys Ram
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5800, 0x5fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5800, 0x5fff));

        // Bank regs
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5ff8, 0x5fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5ff8, 0x5fff));

        if self.file.chips.namco163() {
            cpu.register_read(DeviceKind::Mapper, RangeAndMask(0x4800, 0x6000, 0xffff));
            cpu.register_write(DeviceKind::Mapper, RangeAndMask(0x4800, 0x6000, 0xffff));
        }

        if self.file.chips.mmc5() {
            // Sound regs
            cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xffe0, 0x5000, 0x501f));
            cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xffe0, 0x5000, 0x501f));

            // Misc Regs
            cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5200, 0x5207));
            cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5200, 0x5207));

            // MMC5 EXRAM
            cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5c00, 0x5fff));
            cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5c00, 0x5fff));
        }

        if self.file.chips.fds() {
            cpu.register_read(DeviceKind::Mapper, RangeAndMask(0x4020, 0x4100, 0xffff));
            cpu.register_write(DeviceKind::Mapper, RangeAndMask(0x4020, 0x4100, 0xffff));
        }
    }

    fn peek(&self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.peek_cpu(addr),
            BusKind::Ppu => self.peek_ppu(addr),
        }
    }

    fn read(&mut self, bus: BusKind, addr: u16) -> u8 {
        match bus {
            BusKind::Cpu => self.read_cpu(addr),
            BusKind::Ppu => self.peek_ppu(addr),
        }
    }

    fn write(&mut self, bus: BusKind, addr: u16, value: u8) {
        match bus {
            BusKind::Cpu => self.write_cpu(addr, value),
            BusKind::Ppu => (),
        }
    }

    fn peek_ppu_fetch(&self, _address: u16, _kind: PpuFetchKind) -> super::Nametable {
        super::Nametable::External
    }

    fn tick(&mut self) {
        self.play_timer = self.play_timer.saturating_sub(1);
        if self.play_timer == 0 {
            self.play_pending = true;
            self.play_timer = self.play_timer_load;
        }

        if self.file.chips.vrc7() {
            self.vrc7.tick();
        }

        if self.file.chips.vrc6() {
            self.vrc6.tick();
        }

        if self.file.chips.namco163() {
            self.namco163.tick();
        }

        if self.file.chips.sunsoft5b() {
            self.sunsoft5b.tick();
        }

        if self.file.chips.mmc5() {
            self.mmc5.tick();
        }

        if self.file.chips.fds() {
            self.fds.tick();
        }
    }

    fn get_sample(&self) -> Option<i16> {
        let mut sample = 0;
        let mut count = 0;

        if self.file.chips.vrc7() {
            sample += self.vrc7.output() as i32;
            count += 1;
        }

        if self.file.chips.vrc6() {
            sample += self.vrc6.output() as i32;
            count += 1;
        }

        if self.file.chips.namco163() {
            sample += self.namco163.output() as i32;
            count += 1;
        }

        if self.file.chips.sunsoft5b() {
            sample += self.sunsoft5b.output() as i32;
            count += 1;
        }

        if self.file.chips.mmc5() {
            sample += self.mmc5.output() as i32;
            count += 1;
        }

        if self.file.chips.fds() {
            sample += self.fds.output() as i32;
            count += 1;
        }

        if count > 0 {
            Some((sample / count) as i16)
        } else {
            None
        }
    }

    fn power(&mut self) {
        self.play_timer = self.play_timer_load;
        self.play_pending = false;
        self.current_song = self.file.starting_song.saturating_sub(1);

        for a in 0..0x400u16 {
            self.sys_nt_ram.write(a, 0x00);
        }

        text_line(&mut self.sys_nt_ram, 0, None);
        text_line(&mut self.sys_nt_ram, 1, None);
        text_line(&mut self.sys_nt_ram, 2, Some("Title:"));
        text_line(&mut self.sys_nt_ram, 3, self.file.song_name.as_deref());
        text_line(&mut self.sys_nt_ram, 4, None);
        text_line(&mut self.sys_nt_ram, 5, Some("Artist:"));
        text_line(&mut self.sys_nt_ram, 6, self.file.artist_name.as_deref());
        text_line(&mut self.sys_nt_ram, 7, None);
        text_line(&mut self.sys_nt_ram, 8, Some("Copyright:"));
        text_line(&mut self.sys_nt_ram, 9, self.file.copyright_name.as_deref());
        text_line(&mut self.sys_nt_ram, 10, None);
        text_line(&mut self.sys_nt_ram, 11, None);

        text_line(&mut self.sys_nt_ram, 12, Some("Total Tracks:"));
        number_line(&mut self.sys_nt_ram, 13, Some(self.file.total_songs));
        text_line(&mut self.sys_nt_ram, 14, None);
    }

    #[cfg(feature = "debugger")]
    fn watch(&self, visitor: &mut crate::debug::WatchVisitor) {
        let mut visitor = visitor.group("NSF");
        visitor.value("Load Address", self.file.load_addr);
        visitor.value("Init. Address", self.file.init_addr);
        visitor.value("Play Address", self.file.play_addr);
        visitor.value("Play Rate", self.play_timer_load);
        if let Some(banks) = self.file.init_banks {
            visitor.list("Init. Banks", &banks);
        }
        if let Some(banks) = self.banks {
            visitor.list("Current Banks", &banks);
        }
    }
}

fn number_line(nt: &mut FixedMemoryBlock<1>, line: u16, num: Option<u8>) {
    let mut frac = 100;
    let mut empty = true;
    for x in 0..30 {
        let addr = line * 32 + (x + 2);
        let addr = addr & 0x3ff;

        let value = if let Some(num) = num
            && frac > 0
        {
            let n = (num / frac) % 10;
            empty &= n == 0;
            if frac > 1 && empty { 0x00 } else { n + 0x30 }
        } else {
            0x00
        };

        if frac > 0 {
            frac /= 10;
        }

        nt.write(addr, value);
    }
}

fn text_line(nt: &mut FixedMemoryBlock<1>, line: u16, text: Option<&str>) {
    for x in 0..30 {
        let addr = line * 32 + (x + 2);
        let addr = addr & 0x3ff;

        let value = if let Some(text) = text {
            text.as_bytes().get(x as usize).copied().unwrap_or(0x00)
        } else {
            0x00
        };

        nt.write(addr, value);
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct Vrc6 {
    pulse_a: Vrc6Pulse,
    pulse_b: Vrc6Pulse,
    sawtooth: Sawtooth,
    freq_mode: FreqMode,
    halt_audio: bool,
    mix: i16,
}

impl Vrc6 {
    fn new() -> Self {
        let mix = (i16::MAX as f32 / 64.0) as i16;
        Self {
            pulse_a: Vrc6Pulse::new(),
            pulse_b: Vrc6Pulse::new(),
            sawtooth: Sawtooth::new(),
            freq_mode: FreqMode::X1,
            halt_audio: false,
            mix,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
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
            0xb000 => self.sawtooth.accumulator_rate(value),
            0xb001 => self.sawtooth.freq_low(value),
            0xb002 => self.sawtooth.freq_high(value),
            _ => (),
        }
    }

    fn tick(&mut self) {
        if !self.halt_audio {
            self.pulse_a.tick(self.freq_mode);
            self.pulse_b.tick(self.freq_mode);
            self.sawtooth.tick(self.freq_mode);
        }
    }

    fn output(&self) -> i16 {
        let val = (self.pulse_a.sample() as i16
            + self.pulse_b.sample() as i16
            + self.sawtooth.sample() as i16)
            * self.mix;
        val
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
struct Mmc5 {
    #[cfg_attr(feature = "save-states", save(skip))]
    pulse_table: Vec<i16>,
    #[cfg_attr(feature = "save-states", save(nested))]
    pulse_1: Mmc5Pulse,
    #[cfg_attr(feature = "save-states", save(nested))]
    pulse_2: Mmc5Pulse,
    pcm: Pcm,
}

impl Mmc5 {
    fn new() -> Self {
        let mut pulse_table = Vec::new();
        for x in 0..32 {
            let f_val = 95.52 / (8128.0 / (x as f64) + 100.0);
            pulse_table.push((f_val * ::std::i16::MAX as f64) as i16);
        }

        Self {
            pulse_table,
            pulse_1: Mmc5Pulse::new(),
            pulse_2: Mmc5Pulse::new(),
            pcm: Pcm::new(),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x5010 => {
                let mut value = 0;
                if !self.pcm.write_mode {
                    value |= 0x01;
                }
                value
            }
            0x5015 => {
                let mut value = 0;
                if self.pulse_1.get_state() {
                    value |= 0x01;
                }
                if self.pulse_2.get_state() {
                    value |= 0x02;
                }
                value
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x5000..=0x5003 => self.pulse_1.write(addr & 3, value),
            0x5004..=0x5007 => self.pulse_2.write(addr & 3, value),
            0x5010 => {
                self.pcm.write_mode = value & 0x01 == 0;
            }
            0x5011 => self.pcm.write(value),
            0x5015 => {
                if value & 0x01 != 0 {
                    self.pulse_1.enable();
                } else {
                    self.pulse_1.disable();
                }
                if value & 0x02 != 0 {
                    self.pulse_2.enable();
                } else {
                    self.pulse_2.disable();
                }
            }
            _ => (),
        }
    }

    fn tick(&mut self) {
        self.pulse_1.tick();
        self.pulse_2.tick();
    }

    fn output(&self) -> i16 {
        let pulse_1 = self.pulse_1.output() as usize;
        let pulse_2 = self.pulse_2.output() as usize;

        let out = self.pulse_table[pulse_1 + pulse_2] + self.pcm.output() as i16;
        out
    }
}
