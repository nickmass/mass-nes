#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use std::rc::Rc;

use crate::bus::{AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::{CartMirroring, INes};
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{FixedMemoryBlock, Memory, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::Nametable;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum VertSplitSide {
    Left,
    Right,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum ChrRegSet {
    Sprite,
    Bg,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PpuRead {
    Sprite,
    Bg,
    Nametable,
    Attribute,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum MmcNametable {
    InternalA,
    InternalB,
    Exram,
    Fill,
}

#[derive(Debug, Copy, Clone)]
enum Prg {
    Ram { bank: usize, size: usize },
    Rom { bank: usize, size: usize },
}

impl From<u8> for MmcNametable {
    fn from(value: u8) -> Self {
        match value {
            0x0 => MmcNametable::InternalA,
            0x1 => MmcNametable::InternalB,
            0x2 => MmcNametable::Exram,
            0x3 => MmcNametable::Fill,
            _ => unreachable!(),
        }
    }
}

impl From<MmcNametable> for Nametable {
    fn from(value: MmcNametable) -> Self {
        match value {
            MmcNametable::InternalA => Nametable::InternalA,
            MmcNametable::InternalB => Nametable::InternalB,
            MmcNametable::Exram => Nametable::External,
            MmcNametable::Fill => Nametable::External,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct PpuState {
    last_address: Option<u16>,
    match_count: u8,
    in_frame: bool,
    line_fetches: u8,
    scanline: u8,
    scanline_compare: u8,
    irq_pending: bool,
    reading: bool,
    idle_ticks: u8,
}

impl PpuState {
    fn new() -> Self {
        Self {
            last_address: None,
            match_count: 0,
            in_frame: false,
            line_fetches: 0,
            scanline: 0,
            scanline_compare: 0,
            irq_pending: false,
            reading: false,
            idle_ticks: 0,
        }
    }

    fn fetch(&mut self, addr: u16) {
        self.line_fetches = self.line_fetches.saturating_add(1);

        if addr >= 0x2000 && addr <= 0x2fff && Some(addr) == self.last_address {
            self.match_count += 1;
            if self.match_count == 2 {
                if self.in_frame {
                    self.scanline += 1;
                    if self.scanline == self.scanline_compare {
                        self.irq_pending = true;
                    }
                } else {
                    self.in_frame = true;
                    self.scanline = 0;
                }

                self.line_fetches = 0;
            }
        } else {
            self.match_count = 0;
        }

        self.last_address = Some(addr);
        self.reading = true;
    }

    fn read(&self) -> Option<PpuRead> {
        if !self.in_frame {
            return None;
        }

        let read = match self.line_fetches {
            fetches if fetches < 128 => match fetches & 3 {
                0 => PpuRead::Nametable,
                1 => PpuRead::Attribute,
                2 => PpuRead::Bg,
                3 => PpuRead::Bg,
                _ => unreachable!(),
            },
            fetches if fetches >= 128 && fetches < 160 => match fetches & 3 {
                0 => PpuRead::Nametable,
                1 => PpuRead::Nametable,
                2 => PpuRead::Sprite,
                3 => PpuRead::Sprite,
                _ => unreachable!(),
            },
            fetches if fetches >= 160 && fetches < 168 => match fetches & 3 {
                0 => PpuRead::Nametable,
                1 => PpuRead::Attribute,
                2 => PpuRead::Bg,
                3 => PpuRead::Bg,
                _ => unreachable!(),
            },
            fetches if fetches >= 168 && fetches < 170 => PpuRead::Nametable,
            _ => return None,
        };

        Some(read)
    }

    fn tile_number(&self) -> Option<u8> {
        let fetches = self.line_fetches;
        if fetches < 128 {
            Some((fetches / 4) + 2)
        } else if fetches < 160 {
            None
        } else if fetches < 168 {
            Some((fetches - 160) / 4)
        } else {
            None
        }
    }

    fn tick(&mut self) {
        if self.reading {
            self.idle_ticks = 0;
        } else {
            self.idle_ticks = self.idle_ticks.saturating_add(1);
            if self.idle_ticks == 3 {
                self.leave_frame();
            }
        }
        self.reading = false;
    }

    fn leave_frame(&mut self) {
        self.in_frame = false;
        self.irq_pending = false;
        self.scanline = 0;
        self.last_address = None;
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum PrgRamChips {
    _8k,
    _16k,
    _32k,
    _64k,
    _128k,
}

impl PrgRamChips {
    fn new(kb: usize) -> Self {
        match kb {
            8 => Self::_8k,
            16 => Self::_16k,
            32 => Self::_32k,
            128 => Self::_128k,
            _ => Self::_64k,
        }
    }

    fn map_bank(&self, bank: usize) -> usize {
        match self {
            PrgRamChips::_8k => 0,
            PrgRamChips::_16k => {
                if bank & 0x4 == 0 {
                    0
                } else {
                    1
                }
            }
            PrgRamChips::_32k => bank & 0x3,
            PrgRamChips::_64k => bank & 0x7,
            PrgRamChips::_128k => bank & 0xf,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Mmc5 {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    prg_ram: MemoryBlock,
    prg_ram_chips: PrgRamChips,
    chr_ram: Option<FixedMemoryBlock<8>>,
    exram: FixedMemoryBlock<1>,
    tall_sprites: bool,
    ppu_substitution: bool,
    prg_bank_mode: u8,
    chr_bank_mode: u8,
    ex_ram_mode: u8,
    prg_ram_protect_a: bool,
    prg_ram_protect_b: bool,
    mirroring: [MmcNametable; 4],
    fill_tile: u8,
    fill_attr: u8,
    prg_regs: [u8; 5],
    chr_regs: [u8; 12],
    chr_last_regs: ChrRegSet,
    chr_hi: u8,
    mul_left: u8,
    mul_right: u8,
    irq_enabled: bool,
    ppu_state: PpuState,
    vert_split_threshold: u8,
    vert_split_side: VertSplitSide,
    vert_split_enabled: bool,
    vert_split_scroll: u8,
    vert_chr_bank: u8,
    ex_attr_bank: u8,
    ex_attr_pal: u8,
    #[cfg_attr(feature = "save-states", save(nested))]
    sound: Sound,
}

impl Mmc5 {
    pub fn new(mut cartridge: INes, debug: Rc<Debug>) -> Self {
        let prg_ram_count = if cartridge.prg_ram_bytes == 0 {
            64
        } else {
            cartridge.prg_ram_bytes / 1024
        };

        let prg_ram_chips = PrgRamChips::new(prg_ram_count);
        let mut prg_ram = MemoryBlock::new(prg_ram_count);
        if let Some(wram) = cartridge.wram.take() {
            prg_ram.restore_wram(wram);
        }

        let exram = FixedMemoryBlock::new();

        let chr_ram = if cartridge.chr_ram_bytes > 0 {
            Some(FixedMemoryBlock::new())
        } else {
            None
        };

        use MmcNametable as M;
        let mirroring = match cartridge.mirroring {
            _ if cartridge.alternative_mirroring => {
                [M::InternalA, M::InternalB, M::Exram, M::Exram]
            }
            CartMirroring::Horizontal => [M::InternalA, M::InternalA, M::InternalB, M::InternalB],
            CartMirroring::Vertical => [M::InternalA, M::InternalB, M::InternalA, M::InternalB],
        };

        Self {
            cartridge,
            debug,
            prg_ram,
            prg_ram_chips,
            chr_ram,
            exram,
            tall_sprites: false,
            ppu_substitution: false,
            prg_bank_mode: 3,
            chr_bank_mode: 3,
            ex_ram_mode: 3,
            prg_ram_protect_a: true,
            prg_ram_protect_b: true,
            mirroring,
            fill_tile: 0,
            fill_attr: 0,
            prg_regs: [0xff; 5],
            chr_regs: [0; 12],
            chr_last_regs: ChrRegSet::Sprite,
            chr_hi: 0,
            mul_left: 0xff,
            mul_right: 0xff,
            irq_enabled: false,
            ppu_state: PpuState::new(),
            vert_split_threshold: 0,
            vert_split_side: VertSplitSide::Left,
            vert_split_enabled: false,
            vert_split_scroll: 0,
            vert_chr_bank: 0,
            ex_attr_bank: 0,
            ex_attr_pal: 0,
            sound: Sound::new(),
        }
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
            0x5c00..=0x5fff => self.exram.read(addr),
            0x6000.. => {
                let prg = self.map_prg(addr);
                match prg {
                    Prg::Ram { bank, size } => self.prg_ram.read_mapped(bank, size, addr),
                    Prg::Rom { bank, size } => self.cartridge.prg_rom.read_mapped(bank, size, addr),
                }
            }
            _ => 0,
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
        if let Some(value) = self.sound.read(addr) {
            return value;
        }

        match addr {
            0x5204 => {
                let mut val = 0;
                if self.ppu_state.irq_pending {
                    val |= 0x80;
                }
                if self.ppu_state.in_frame {
                    val |= 0x40;
                }
                self.ppu_state.irq_pending = false;
                val
            }
            0x5205 => {
                let val = self.mul_left as u16 * self.mul_right as u16;
                val as u8
            }
            0x5206 => {
                let val = self.mul_left as u16 * self.mul_right as u16;
                (val >> 8) as u8
            }
            0x5c00..=0x7fff => self.peek_cpu(addr),
            0x8000.. => {
                if addr == 0xfffa || addr == 0xfffb {
                    self.ppu_state.leave_frame();
                }
                let value = self.peek_cpu(addr);
                self.sound.pcm_read(addr, value);
                value
            }
            _ => 0,
        }
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        self.sound.write(addr, value);

        match addr {
            0x2000 => {
                self.tall_sprites = value & 0x20 != 0;
                if !self.tall_sprites {
                    self.chr_last_regs = ChrRegSet::Sprite;
                }
            }
            0x2001 => {
                self.ppu_substitution = value & 0x18 != 0;
                if !self.ppu_substitution {
                    self.ppu_state.leave_frame();
                }
            }
            0x5100 => self.prg_bank_mode = value & 0x3,
            0x5101 => self.chr_bank_mode = value & 0x3,
            0x5102 => self.prg_ram_protect_a = value & 0x3 != 0x2,
            0x5103 => self.prg_ram_protect_b = value & 0x3 != 0x1,
            0x5104 => self.ex_ram_mode = value & 0x3,
            0x5105 => {
                let mut value = value;
                for nt in self.mirroring.iter_mut() {
                    let table = value & 3;
                    *nt = table.into();
                    value >>= 2;
                }
            }
            0x5106 => self.fill_tile = value,
            0x5107 => self.fill_attr = (value & 0x3) * 0b01010101,
            0x5113..=0x5117 => {
                let prg_reg_idx = addr - 0x5113;
                self.prg_regs[prg_reg_idx as usize] = value;
            }
            0x5120..=0x512b => {
                let chr_reg_idx = addr - 0x5120;
                self.chr_last_regs = if chr_reg_idx <= 7 {
                    ChrRegSet::Sprite
                } else {
                    ChrRegSet::Bg
                };
                self.chr_regs[chr_reg_idx as usize] = value;
            }
            0x5130 => self.chr_hi = value & 0x3,
            0x5200 => {
                self.vert_split_enabled = value & 0x80 != 0;
                self.vert_split_side = if value & 0x40 != 0 {
                    VertSplitSide::Right
                } else {
                    VertSplitSide::Left
                };
                self.vert_split_threshold = value & 0x1f;
            }
            0x5201 => self.vert_split_scroll = value,
            0x5202 => self.vert_chr_bank = value,
            0x5203 => self.ppu_state.scanline_compare = value,
            0x5204 => self.irq_enabled = value & 0x80 != 0,
            0x5205 => self.mul_left = value,
            0x5206 => self.mul_right = value,
            0x5c00..=0x5fff => {
                if self.ex_ram_mode != 3 {
                    self.exram.write(addr, value)
                }
            }
            0x6000.. => {
                if self.prg_ram_protect_a || self.prg_ram_protect_b {
                    return;
                }
                let prg = self.map_prg(addr);
                if let Prg::Ram { bank, size } = prg {
                    self.prg_ram.write_mapped(bank, size, addr, value);
                }
            }
            _ => (),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if let Some(chr_ram) = self.chr_ram.as_ref() {
            if addr < 0x2000 {
                return chr_ram.read(addr);
            }
        }
        if self.in_vert_split() {
            match self.ppu_state.read() {
                Some(PpuRead::Nametable) => self.vert_split_nt(),
                Some(PpuRead::Attribute) => self.vert_split_attr(),
                Some(PpuRead::Bg) => {
                    let bank = self.vert_chr_bank as usize | (self.chr_hi as usize) << 8;
                    self.cartridge.chr_rom.read_mapped(bank, 4 * 1024, addr)
                }
                _ => 0x00,
            }
        } else if addr < 0x2000 {
            if !(self.tall_sprites || self.ex_ram_mode == 0x01) || !self.ppu_substitution {
                self.read_chr(addr)
            } else {
                match (self.ppu_state.read(), self.chr_last_regs) {
                    (Some(PpuRead::Sprite), _) => self.read_chr(addr),
                    (Some(PpuRead::Bg), _) if self.ex_ram_mode == 0x01 => {
                        let bank = self.ex_attr_bank as usize | (self.chr_hi as usize) << 6;
                        self.cartridge.chr_rom.read_mapped(bank, 4 * 1024, addr)
                    }
                    (Some(PpuRead::Bg), _) => self.read_ext_chr(addr),
                    (_, ChrRegSet::Sprite) => self.read_chr(addr),
                    (_, ChrRegSet::Bg) => self.read_ext_chr(addr),
                }
            }
        } else {
            let read = self.ppu_state.read();
            if read == Some(PpuRead::Attribute) && self.ppu_substitution && self.ex_ram_mode == 0x01
            {
                return self.ex_attr_pal;
            }

            let table = (addr & 0xc00) >> 10;
            match self.mirroring[table as usize] {
                MmcNametable::Exram => match self.ex_ram_mode {
                    0 | 1 => self.exram.read(addr),
                    _ => 0,
                },
                MmcNametable::Fill => match read {
                    Some(PpuRead::Attribute) => self.fill_attr,
                    _ => self.fill_tile,
                },
                _ => 0,
            }
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let (bank, size) = self.map_chr(addr);
        self.cartridge.chr_rom.read_mapped(bank, size, addr)
    }

    fn read_ext_chr(&self, addr: u16) -> u8 {
        let (bank, size) = self.map_ext_chr(addr);
        self.cartridge.chr_rom.read_mapped(bank, size, addr)
    }

    fn write_ppu(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            if let Some(chr_ram) = self.chr_ram.as_mut() {
                chr_ram.write(addr, val);
            }
            return;
        }

        let table = (addr & 0xc00) >> 10;
        if let MmcNametable::Exram = self.mirroring[table as usize] {
            if self.ex_ram_mode == 0 || self.ex_ram_mode == 1 {
                self.exram.write(addr, val);
            }
        }
    }

    fn map_prg(&self, addr: u16) -> Prg {
        if addr & 0x8000 == 0 {
            let bank = self.prg_regs[0] as usize & 0xf;
            Prg::Ram {
                bank: self.prg_ram_chips.map_bank(bank),
                size: 8 * 1024,
            }
        } else {
            let (bank_idx, size) = match self.prg_bank_mode {
                0x00 => (4, 32),
                0x01 if addr & 0xc000 == 0x8000 => (2, 16),
                0x01 if addr & 0xc000 == 0xc000 => (4, 16),
                0x02 if addr & 0xc000 == 0x8000 => (2, 16),
                0x02 if addr & 0xe000 == 0xc000 => (3, 8),
                0x02 if addr & 0xe000 == 0xe000 => (4, 8),
                0x03 if addr & 0xe000 == 0x8000 => (1, 8),
                0x03 if addr & 0xe000 == 0xa000 => (2, 8),
                0x03 if addr & 0xe000 == 0xc000 => (3, 8),
                0x03 if addr & 0xe000 == 0xe000 => (4, 8),
                _ => unreachable!(),
            };

            let bank = self.prg_regs[bank_idx] as usize;
            let ram = bank & 0x80 == 0 && bank_idx != 4;
            let bank = if ram {
                self.prg_ram_chips.map_bank(bank & 0xf)
            } else {
                bank & 0x7f
            };

            let bank = match size {
                32 => bank >> 2,
                16 => bank >> 1,
                _ => bank,
            };

            let size = size * 1024;

            if ram {
                Prg::Ram { bank, size }
            } else {
                Prg::Rom { bank, size }
            }
        }
    }

    fn map_chr(&self, addr: u16) -> (usize, usize) {
        let addr = addr as usize & 0x1fff;
        let (bank_idx, size) = match self.chr_bank_mode {
            0x00 => (7, 8),
            0x01 => (addr >> 10 | 3, 4),
            0x02 => (addr >> 10 | 1, 2),
            0x03 => (addr >> 10, 1),
            _ => unreachable!(),
        };

        let bank = self.chr_regs[bank_idx] as usize | (self.chr_hi as usize) << 8;

        (bank, size * 1024)
    }

    fn map_ext_chr(&self, addr: u16) -> (usize, usize) {
        let addr = addr as usize & 0x0fff;
        let (bank_idx, size) = match self.chr_bank_mode {
            0x00 => (11, 8),
            0x01 => (11, 4),
            0x02 => (addr & 0x800 >> 10 | 9, 2),
            0x03 => (addr >> 10 | 8, 1),
            _ => unreachable!(),
        };

        let bank = self.chr_regs[bank_idx] as usize | (self.chr_hi as usize) << 8;

        (bank, size * 1024)
    }

    fn in_vert_split(&self) -> bool {
        let enabled = self.vert_split_enabled
            && (self.ex_ram_mode == 0 || self.ex_ram_mode == 1)
            && self.ppu_state.in_frame
            && self.ppu_substitution;

        if !enabled {
            return false;
        }

        if let Some(tile) = self.ppu_state.tile_number() {
            match self.vert_split_side {
                VertSplitSide::Left => tile < self.vert_split_threshold,
                VertSplitSide::Right => tile >= self.vert_split_threshold,
            }
        } else {
            false
        }
    }

    fn vert_split_nt(&self) -> u8 {
        let row = ((self.ppu_state.scanline as u16 + self.vert_split_scroll as u16) / 8) % 30;
        let col = (self.ppu_state.tile_number().unwrap_or(0) as u16) % 32;
        let tile_idx = row * 32 + col;

        self.exram.read(tile_idx)
    }

    fn vert_split_attr(&self) -> u8 {
        let row = ((self.ppu_state.scanline as u16 + self.vert_split_scroll as u16) / 8) % 30;
        let col = (self.ppu_state.tile_number().unwrap_or(0) as u16) % 32;
        let tile_idx = row * 32 + col;
        let attr_addr = 0x3c0 | (tile_idx >> 4 & 0x38) | (tile_idx >> 2 & 7);

        self.exram.read(attr_addr)
    }
}

impl Mapper for Mmc5 {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));

        // Sound regs
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xffe0, 0x5000, 0x501f));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xffe0, 0x5000, 0x501f));

        // Config & PRG/CHR Banks
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x5100, 0x51ff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x5100, 0x51ff));

        // Misc Regs
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5200, 0x5207));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x5200, 0x5207));

        // EXRAM
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5c00, 0x5fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfc00, 0x5c00, 0x5fff));

        // PPU reg watch
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xfff8, 0x2000, 0x2007));
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

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> super::Nametable {
        if self.in_vert_split() {
            Nametable::External
        } else if address & 0x2000 != 0 {
            let table = (address & 0xc00) >> 10;
            self.mirroring[table as usize].into()
        } else {
            Nametable::External
        }
    }

    fn get_irq(&self) -> bool {
        (self.ppu_state.irq_pending && self.irq_enabled) || self.sound.pcm_irq()
    }

    fn tick(&mut self) {
        self.ppu_state.tick();
        self.sound.tick();
    }

    fn ppu_fetch(&mut self, address: u16, kind: PpuFetchKind) -> super::Nametable {
        if kind != PpuFetchKind::Idle {
            let was_irq = self.ppu_state.irq_pending;
            self.ppu_state.fetch(address);
            if self.irq_enabled && !was_irq && self.ppu_state.irq_pending {
                self.debug.event(crate::DebugEvent::MapperIrq);
            }
        }

        if self.ex_ram_mode == 0x01 && self.ppu_substitution {
            if let Some(state) = self.ppu_state.read() {
                match state {
                    PpuRead::Nametable => {
                        let val = self.exram.read(address);
                        self.ex_attr_bank = val & 0x3f;
                        self.ex_attr_pal = (val >> 6) * 0b01010101;
                    }
                    PpuRead::Bg => return Nametable::External,
                    PpuRead::Attribute => return Nametable::External,
                    PpuRead::Sprite => (),
                }
            }
        }

        self.peek_ppu_fetch(address, kind)
    }

    fn get_sample(&self) -> Option<i16> {
        Some(self.sound.output())
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg_ram.save_wram()
        } else {
            None
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Default)]
struct Pulse {
    period: u16,
    timer_counter: u16,
    length_counter: u8,
    sequencer: u8,
    enabled: bool,
    envelope_start: bool,
    envelope_divider: u8,
    decay_counter: u8,
    regs: [u8; 4],
    current_tick: u64,
}

impl Pulse {
    fn new() -> Pulse {
        Pulse {
            ..Default::default()
        }
    }

    fn timer_load(&self) -> u16 {
        (self.regs[2] as u16) | ((self.regs[3] as u16 & 7) << 8)
    }

    fn length_load(&self) -> u8 {
        if !self.enabled {
            0
        } else {
            crate::apu::LENGTH_TABLE[(self.regs[3] >> 3 & 0x1f) as usize]
        }
    }

    fn envelope_volume(&self) -> u8 {
        self.regs[0] & 0xf
    }

    fn envelope_output(&self) -> u8 {
        if self.constant_volume() {
            self.envelope_volume()
        } else {
            self.decay_counter
        }
    }

    fn constant_volume(&self) -> bool {
        self.regs[0] & 0x10 != 0
    }

    fn halt(&self) -> bool {
        self.regs[0] & 0x20 != 0
    }

    fn duty_sequence(&self) -> [bool; 8] {
        match self.regs[0] >> 6 & 3 {
            0 => [false, true, false, false, false, false, false, false],
            1 => [false, true, true, false, false, false, false, false],
            2 => [false, true, true, true, true, false, false, false],
            3 => [true, false, false, true, true, true, true, true],
            _ => unreachable!(),
        }
    }

    fn duty(&self) -> bool {
        self.duty_sequence()[(self.sequencer & 7) as usize]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.regs[addr as usize] = value;
        match addr {
            0 => (),
            1 => (),
            2 => {
                self.period = self.timer_load();
            }
            3 => {
                self.period = self.timer_load();
                self.sequencer = 0;
                self.length_counter = self.length_load();
                self.envelope_start = true;
            }
            _ => unreachable!(),
        }
    }

    fn tick(&mut self) {
        self.current_tick += 1;

        if self.current_tick & 1 == 0 {
            if self.timer_counter == 0 {
                self.timer_counter = self.period;
                self.sequencer = self.sequencer.wrapping_add(1);
            } else {
                self.timer_counter -= 1;
            }
        }

        if self.current_tick == 7456 {
            self.current_tick = 0;
            if self.envelope_start {
                self.envelope_start = false;
                self.decay_counter = 0xf;
                self.envelope_divider = self.envelope_volume();
            } else if self.envelope_divider == 0 {
                self.envelope_divider = self.envelope_volume();
                if self.decay_counter == 0 {
                    if self.halt() {
                        self.decay_counter = 0xf
                    }
                } else {
                    self.decay_counter -= 1;
                }
            } else {
                self.envelope_divider -= 1;
            }
            if self.length_counter != 0 && !self.halt() {
                self.length_counter -= 1;
            }
        }
    }

    fn output(&self) -> u8 {
        if !self.duty() || self.length_counter == 0 {
            0
        } else {
            self.envelope_output()
        }
    }

    fn enable(&mut self) {
        self.enabled = true;
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
    }

    fn get_state(&self) -> bool {
        self.length_counter > 0
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Pcm {
    output: u8,
    write_mode: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Pcm {
    fn new() -> Self {
        Self {
            output: 0,
            write_mode: true,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    fn output(&self) -> u8 {
        self.output
    }

    fn read(&mut self, addr: u16, value: u8) {
        if self.write_mode {
            return;
        }

        if addr < 0x8000 || addr >= 0xc000 {
            return;
        }

        if value == 0 {
            self.irq_pending = true;
        } else {
            self.output = value;
        }
    }

    fn write(&mut self, value: u8) {
        if !self.write_mode {
            return;
        }

        if value == 0 {
            self.irq_pending = true;
        } else {
            self.output = value;
        }
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Sound {
    #[cfg_attr(feature = "save-states", save(skip))]
    pulse_table: Vec<i16>,
    #[cfg_attr(feature = "save-states", save(nested))]
    pulse_1: Pulse,
    #[cfg_attr(feature = "save-states", save(nested))]
    pulse_2: Pulse,
    pcm: Pcm,
}

impl Sound {
    pub fn new() -> Self {
        let mut pulse_table = Vec::new();
        for x in 0..32 {
            let f_val = 95.52 / (8128.0 / (x as f64) + 100.0);
            pulse_table.push((f_val * ::std::i16::MAX as f64) as i16);
        }

        Self {
            pulse_table,
            pulse_1: Pulse::new(),
            pulse_2: Pulse::new(),
            pcm: Pcm::new(),
        }
    }

    pub fn read(&mut self, addr: u16) -> Option<u8> {
        let value = match addr {
            0x5010 => {
                let mut value = 0;
                if !self.pcm.write_mode {
                    value |= 0x01;
                }
                if self.pcm.irq_enabled && self.pcm.irq_pending {
                    value |= 0x80;
                }
                self.pcm.irq_pending = false;
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
            _ => return None,
        };

        Some(value)
    }

    pub fn pcm_read(&mut self, addr: u16, value: u8) {
        self.pcm.read(addr, value);
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x5000..=0x5003 => self.pulse_1.write(addr & 3, value),
            0x5004..=0x5007 => self.pulse_2.write(addr & 3, value),
            0x5010 => {
                self.pcm.write_mode = value & 0x01 == 0;
                self.pcm.irq_enabled = value & 0x80 != 0;
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

    pub fn tick(&mut self) {
        self.pulse_1.tick();
        self.pulse_2.tick();
    }

    pub fn pcm_irq(&self) -> bool {
        self.pcm.irq_pending && self.pcm.irq_enabled
    }

    pub fn output(&self) -> i16 {
        let pulse_1 = self.pulse_1.output() as usize;
        let pulse_2 = self.pulse_2.output() as usize;

        let out = self.pulse_table[pulse_1 + pulse_2] + self.pcm.output() as i16;
        out
    }
}
