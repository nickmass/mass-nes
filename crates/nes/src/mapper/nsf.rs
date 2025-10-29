#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use super::fds::Sound as Fds;
use super::fme7::Sound as Sunsoft5b;
use super::mmc5::Sound as Mmc5;
use super::namco163::Sound as Namco163;
use super::vrc6::Sound as Vrc6;
use super::vrc7::Sound as Vrc7;
use crate::Region;
use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::NsfFile;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory, MemoryBlock, RomBlock};
use crate::ppu::PpuFetchKind;

use std::fmt::Write;

static NSF_PLAYER_ROM: &[u8] = include_bytes!("nsf_player/nsf_player.bin");
static NSF_PLAYER_CHR: &[u8] = include_bytes!("nsf_player/ascii-by-jroatch.chr");

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
    sunsoft5b: Option<Sunsoft5b>,
    namco163: Option<Namco163>,
    vrc6: Option<Vrc6>,
    #[cfg_attr(feature = "save-states", save(nested))]
    vrc7: Option<Vrc7>,
    #[cfg_attr(feature = "save-states", save(nested))]
    mmc5: Option<Mmc5>,
    fds: Option<Fds>,
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
        let current_song = file.starting_song;

        let sunsoft5b = file.chips.sunsoft5b().then(|| Sunsoft5b::new());
        let namco163 = file.chips.namco163().then(|| {
            let mut namco163 = Namco163::new();
            namco163.enable(0x00);
            namco163
        });
        let vrc6 = file.chips.vrc6().then(|| Vrc6::new());
        let vrc7 = file.chips.vrc7().then(|| {
            let mut vrc7 = Vrc7::new();
            vrc7.write(0xe000, 0x00);
            vrc7
        });
        let mmc5 = file.chips.mmc5().then(|| Mmc5::new());
        let fds = file.chips.fds().then(|| {
            let mut fds = Fds::new();
            fds.write(0x4080, 0x80);
            fds.write(0x408a, 0xe8);
            fds
        });

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
            current_song,
            sunsoft5b,
            namco163,
            vrc6,
            vrc7,
            mmc5,
            fds,
        }
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
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
        if let Some(value) = self.sunsoft5b.read(addr) {
            return value;
        } else if let Some(value) = self.namco163.read(addr) {
            return value;
        } else if let Some(value) = self.vrc6.read(addr) {
            return value;
        } else if let Some(value) = self.vrc7.read(addr) {
            return value;
        } else if let Some(value) = self.mmc5.read(addr) {
            return value;
        } else if let Some(value) = self.fds.read(addr) {
            return value;
        }

        match addr {
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
            0xfffc => 0x03,
            0xfffd => 0x54,
            0x8000.. => {
                let value = self.read_prg(addr);
                if let Some(mmc5) = self.mmc5.as_mut() {
                    mmc5.pcm_read(addr, value);
                }
                value
            }
            _ => self.peek_cpu(addr),
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        self.sunsoft5b.write(addr, value);
        self.namco163.write(addr, value);
        self.vrc6.write(addr, value);
        self.vrc7.write(addr, value);
        self.mmc5.write(addr, value);
        self.fds.write(addr, value);

        match addr {
            0x5205 if self.file.chips.mmc5() => self.mul_left = value,
            0x5206 if self.file.chips.mmc5() => self.mul_right = value,
            0x5302 => self.init_song(),
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
            self.sys_chr.read(addr & 0xfff)
        } else {
            self.sys_nt_ram.read(addr & 0x3ff)
        }
    }

    fn init_song(&mut self) {
        self.banks = self.file.init_banks;

        if let Some((fds_banks, banks)) = self.fds_banks.as_mut().zip(self.banks.as_ref()) {
            fds_banks[0] = banks[6];
            fds_banks[1] = banks[7];
        }

        for a in 0..0x2000u16 {
            self.prg_ram.write(a, 0x00);
        }

        let _ = self.display_info();

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

    fn display_info(&mut self) -> std::fmt::Result {
        let mut cursor = NametableCursor::new(&mut self.sys_nt_ram);

        writeln!(cursor)?;
        writeln!(cursor, "NSF Player")?;
        writeln!(cursor)?;

        cursor.write_label("Title:", self.file.song_name.as_ref())?;
        cursor.write_label("Artist:", self.file.artist_name.as_ref())?;
        cursor.write_label("Copyright:", self.file.copyright_name.as_ref())?;

        if self.file.total_songs > 1 {
            cursor.move_to_line(-7);
            let current_track = self.current_song.saturating_add(1);
            writeln!(cursor, "Track {current_track} of {}", self.file.total_songs)?;
            writeln!(cursor)?;
            writeln!(cursor, "Use left/right to change track")?;
        }

        Ok(())
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

        self.sunsoft5b.tick();
        self.namco163.tick();
        self.vrc6.tick();
        self.vrc7.tick();
        self.mmc5.tick();
        self.fds.tick();
    }

    fn get_sample(&self) -> Option<i16> {
        let mut sample = 0;
        let mut count = 0;

        self.sunsoft5b.add_output(&mut count, &mut sample);
        self.namco163.add_output(&mut count, &mut sample);
        self.vrc6.add_output(&mut count, &mut sample);
        self.vrc7.add_output(&mut count, &mut sample);
        self.mmc5.add_output(&mut count, &mut sample);
        self.fds.add_output(&mut count, &mut sample);

        if count > 0 {
            Some((sample / count) as i16)
        } else {
            None
        }
    }

    fn power(&mut self) {
        self.play_timer = self.play_timer_load;
        self.play_pending = false;
        self.current_song = self.file.starting_song;

        for a in 0..0x400u16 {
            self.sys_nt_ram.write(a, 0x00);
        }

        let _ = self.display_info();
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

struct NametableCursor<'a, M: Memory> {
    nt: &'a mut M,
    line: u8,
    column: u8,
}

impl<'a, M: Memory> NametableCursor<'a, M> {
    fn new(nt: &'a mut M) -> Self {
        for a in 0..0x400u16 {
            nt.write(a, 0x00);
        }

        Self {
            nt,
            line: 0,
            column: 1,
        }
    }

    fn write_label(&mut self, label: &str, value: Option<&String>) -> std::fmt::Result {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            if label.len() + value.len() < 30 {
                writeln!(self, "{label} {value}")?;
            } else {
                writeln!(self, "{label}")?;
                writeln!(self, "{value}")?;
            }
            writeln!(self)?;
        }

        Ok(())
    }

    fn move_to_line(&mut self, line: i8) {
        let abs = line.abs() as u8;
        if line < 0 {
            self.line = 31 - abs;
        } else {
            self.line = abs;
        }
        self.column = 1;
    }
}

impl<'a, M: Memory> std::fmt::Write for NametableCursor<'a, M> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }

        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else if c.is_ascii() && !c.is_ascii_control() {
            if self.column < 32 && self.line < 32 {
                let addr = (self.line as u16 * 32 + self.column as u16) & 0x3ff;
                self.nt.write(addr, c as u8);
            }
            self.column += 1;
        }

        Ok(())
    }
}

trait ExternalAudio {
    fn read(&mut self, _address: u16) -> Option<u8> {
        None
    }

    fn write(&mut self, address: u16, value: u8);
    fn tick(&mut self);
    fn output(&self) -> Option<i16>;
    fn add_output(&self, count: &mut i32, accum: &mut i32) {
        if let Some(output) = self.output() {
            *count += 1;
            *accum += output as i32;
        }
    }
}

impl<T: ExternalAudio> ExternalAudio for Option<T> {
    fn read(&mut self, address: u16) -> Option<u8> {
        self.as_mut().and_then(|sound| sound.read(address))
    }

    fn write(&mut self, address: u16, value: u8) {
        if let Some(sound) = self.as_mut() {
            sound.write(address, value);
        }
    }

    fn tick(&mut self) {
        if let Some(sound) = self.as_mut() {
            sound.tick();
        }
    }

    fn output(&self) -> Option<i16> {
        self.as_ref().and_then(|sound| sound.output())
    }
}

impl ExternalAudio for Sunsoft5b {
    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xc000..=0xcfff => self.select(value),
            0xe000..=0xefff => self.value(value),
            _ => (),
        }
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}

impl ExternalAudio for Namco163 {
    fn read(&mut self, address: u16) -> Option<u8> {
        match address {
            0x4800..=0x4fff => Some(self.read()),
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4800..=0x4fff => self.write(value),
            0xf800..=0xffff => self.address_port(value),
            _ => (),
        }
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}

impl ExternalAudio for Vrc6 {
    fn write(&mut self, address: u16, value: u8) {
        self.write(address, value);
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}

impl ExternalAudio for Vrc7 {
    fn write(&mut self, address: u16, value: u8) {
        self.write(address, value);
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}

impl ExternalAudio for Mmc5 {
    fn read(&mut self, address: u16) -> Option<u8> {
        self.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.write(address, value);
    }

    fn tick(&mut self) {
        self.tick()
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}

impl ExternalAudio for Fds {
    fn read(&mut self, address: u16) -> Option<u8> {
        match address {
            0x4040..=0x4097 => Some(self.read(address)),
            _ => None,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        self.write(address, value);
    }

    fn tick(&mut self) {
        self.tick()
    }

    fn output(&self) -> Option<i16> {
        Some(self.output())
    }
}
