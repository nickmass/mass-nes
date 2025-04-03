#[cfg(feature = "save-states")]
use nes_traits::SaveState;

#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use std::rc::Rc;

use super::vrc6::{FreqMode, Pulse, Sawtooth};
use crate::bus::{Address, AddressBus, AndAndMask, AndEqualsAndMask, BusKind, DeviceKind};
use crate::cartridge::INes;
use crate::debug::Debug;
use crate::mapper::Mapper;
use crate::memory::{BankKind, MappedMemory, MemKind, MemoryBlock};
use crate::ppu::PpuFetchKind;

use super::Nametable;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct SpriteExtRegs(#[cfg_attr(feature = "save-states", serde(with = "serde_arrays"))] [u8; 64]);

impl std::ops::Deref for SpriteExtRegs {
    type Target = [u8; 64];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SpriteExtRegs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PpuRead {
    Sprite,
    Bg,
    ExtBg(u8),
    ExtSprite(u8),
    Nametable,
    Attribute,
    ExtAttribute(u8),
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
    irq_offset: u8,
    irq_jitter: u8,
    ext_bg: Option<u8>,
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
            irq_offset: 135,
            irq_jitter: 0,
            ext_bg: None,
        }
    }

    fn fetch(&mut self, addr: u16, nt_mode: Option<u8>, fpga: &MemoryBlock, oam: &mut ShadowOam) {
        self.line_fetches = self.line_fetches.saturating_add(1);

        if self.line_fetches == self.irq_offset
            && self.scanline == self.scanline_compare
            && self.scanline != 0
        {
            self.irq_pending = true;
            self.irq_jitter = 0;
        }

        if addr >= 0x2000 && addr <= 0x2fff && Some(addr) == self.last_address {
            self.match_count += 1;
            if self.match_count == 2 {
                if self.in_frame {
                    self.scanline += 1;
                } else {
                    self.in_frame = true;
                    self.scanline = 0;
                }
                oam.eval(self.scanline);
                self.line_fetches = 0;
            }
        } else {
            self.match_count = 0;
        }

        self.last_address = Some(addr);
        self.reading = true;

        if self.in_frame && self.line_fetches >= 128 && self.line_fetches < 160 {
            oam.oam_addr(0);
        }

        if let Some(nt_mode) = nt_mode {
            if self.in_frame && nt_mode & 0x2 != 0 {
                if self.line_fetches & 3 == 0 {
                    let fpga_bank = (nt_mode >> 2) & 3;
                    let fpga_addr = (fpga_bank as u16 * 0x400) | (addr & 0x3ff);
                    let value = fpga.read(fpga_addr);
                    self.ext_bg = Some(value);
                }
            } else {
                self.ext_bg = None;
            }
        }
    }

    fn read(&self, oam: Option<&ShadowOam>) -> Option<PpuRead> {
        if !self.in_frame {
            return None;
        }

        let read = match self.line_fetches {
            fetches if fetches < 128 => match (fetches & 3, self.ext_bg) {
                (0, _) => PpuRead::Nametable,
                (1, Some(b)) => PpuRead::ExtAttribute((b >> 6) * 0b0101_0101),
                (1, _) => PpuRead::Attribute,
                (2 | 3, Some(b)) => PpuRead::ExtBg(b & 0x3f),
                (2, _) => PpuRead::Bg,
                (3, _) => PpuRead::Bg,
                _ => unreachable!(),
            },
            fetches if fetches >= 128 && fetches < 160 => match (fetches & 3, oam) {
                (0, _) => PpuRead::Nametable,
                (1, _) => PpuRead::Nametable,
                (2 | 3, Some(oam)) => {
                    let idx = (fetches as usize - 128) / 4;
                    let tile = oam.line_oam[idx];
                    PpuRead::ExtSprite(tile)
                }
                (2, _) => PpuRead::Sprite,
                (3, _) => PpuRead::Sprite,
                _ => unreachable!(),
            },
            fetches if fetches >= 160 && fetches < 168 => match (fetches & 3, self.ext_bg) {
                (0, _) => PpuRead::Nametable,
                (1, Some(b)) => PpuRead::ExtAttribute((b >> 6) * 0b0101_0101),
                (1, _) => PpuRead::Attribute,
                (2 | 3, Some(b)) => PpuRead::ExtBg(b & 0x3f),
                (2, _) => PpuRead::Bg,
                (3, _) => PpuRead::Bg,
                _ => unreachable!(),
            },
            fetches if fetches >= 168 && fetches < 170 => PpuRead::Nametable,
            _ => return None,
        };

        Some(read)
    }

    // will be used for Window Split Mode
    #[allow(unused)]
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
        self.irq_jitter = self.irq_jitter.saturating_add(1);
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

    fn hblank(&self) -> bool {
        self.in_frame && self.line_fetches >= 128
    }

    fn leave_frame(&mut self) {
        self.in_frame = false;
        self.irq_pending = false;
        self.scanline = 0;
        self.last_address = None;
        self.ext_bg = None;
    }
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Rainbow {
    #[cfg_attr(feature = "save-states", save(skip))]
    cartridge: INes,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    prg: MappedMemory,
    chr: MappedMemory,
    fpga_ram: MemoryBlock,
    prg_mode: u8,
    prg_lo_regs: [u8; 8],
    prg_hi_regs: [u8; 8],
    prg_ram_lo_regs: [u8; 2],
    prg_ram_hi_regs: [u8; 2],
    fpga_ram_reg: u8,
    chr_mode: u8,
    bg_ext_hi: u8,
    nt_bank_regs: [u8; 5],
    nt_mode_regs: [u8; 5],
    fill_tile: u8,
    fill_attr: u8,
    chr_lo_regs: [u8; 16],
    chr_hi_regs: [u8; 16],
    spr_ext_lo: SpriteExtRegs,
    spr_ext_hi: u8,
    ppu_state: PpuState,
    ppu_irq_enabled: bool,
    oam_state: ShadowOam,
    cpu_irq_latch: u16,
    cpu_irq_counter: u16,
    cpu_irq_pending: bool,
    cpu_irq_enable: bool,
    cpu_irq_ack_enable: bool,
    cpu_irq_4011_ack: bool,
    fpga_reader_addr: u16,
    fpga_reader_inc: u8,
    pulse_a: Pulse,
    pulse_b: Pulse,
    sawtooth: Sawtooth,
    audio_enable: bool,
    audio_4011_out: bool,
    master_volume: i16,
    redir_nmi: bool,
    redir_nmi_lo: u8,
    redir_nmi_hi: u8,
    redir_irq: bool,
    redir_irq_lo: u8,
    redir_irq_hi: u8,
    warn_window: bool,
    warn_512_banks: bool,
}

impl Rainbow {
    pub fn new(mut cartridge: INes, debug: Rc<Debug>) -> Self {
        let mut prg = MappedMemory::new(
            &cartridge,
            0x6000,
            cartridge.prg_ram_bytes as u32 / 1024,
            40,
            MemKind::Prg,
        );

        let chr = MappedMemory::new(
            &cartridge,
            0x0000,
            cartridge.chr_ram_bytes as u32 / 1024,
            8,
            MemKind::Chr,
        );

        if let Some(wram) = cartridge.wram.take() {
            prg.restore_wram(wram);
        }

        let master_volume = (i16::MAX as f32 / 64.0) as i16;

        let mut rom = Self {
            cartridge,
            debug,
            prg,
            chr,
            fpga_ram: MemoryBlock::new(8),
            prg_mode: 0,
            prg_lo_regs: [0; 8],
            prg_hi_regs: [0; 8],
            prg_ram_lo_regs: [0; 2],
            prg_ram_hi_regs: [0; 2],
            fpga_ram_reg: 0,
            chr_mode: 0,
            bg_ext_hi: 0,
            nt_bank_regs: [0, 0, 1, 1, 0],
            nt_mode_regs: [0, 0, 0, 0, 0x80],
            fill_tile: 0,
            fill_attr: 0,
            chr_lo_regs: [0; 16],
            chr_hi_regs: [0; 16],
            spr_ext_lo: SpriteExtRegs([0; 64]),
            spr_ext_hi: 0,
            ppu_state: PpuState::new(),
            ppu_irq_enabled: false,
            oam_state: ShadowOam::new(),
            cpu_irq_latch: 0,
            cpu_irq_counter: 0,
            cpu_irq_pending: false,
            cpu_irq_enable: false,
            cpu_irq_ack_enable: false,
            cpu_irq_4011_ack: false,
            fpga_reader_addr: 0,
            fpga_reader_inc: 1,
            pulse_a: Pulse::new(),
            pulse_b: Pulse::new(),
            sawtooth: Sawtooth::new(),
            audio_enable: true,
            audio_4011_out: false,
            master_volume,
            redir_nmi: false,
            redir_nmi_lo: 0,
            redir_nmi_hi: 0,
            redir_irq: false,
            redir_irq_lo: 0,
            redir_irq_hi: 0,
            warn_window: false,
            warn_512_banks: false,
        };

        rom.sync_prg();
        rom.sync_chr();

        rom
    }

    fn peek_cpu(&self, addr: u16) -> u8 {
        match addr {
            0x4011 if self.audio_4011_out => {
                (self.pulse_a.sample() + self.pulse_b.sample() + self.sawtooth.sample()) << 2
            }
            0x4150 => self.ppu_state.scanline,
            0x4151 => {
                let mut val = 0;
                if self.ppu_state.irq_pending {
                    val |= 1;
                }
                if self.ppu_state.in_frame {
                    val |= 0x40;
                }
                if self.ppu_state.hblank() {
                    val |= 0x80;
                }

                val
            }
            0x4154 => self.ppu_state.irq_jitter,
            0x415f => self.fpga_ram.read(self.fpga_reader_addr),
            0x4160 => 0x20,
            0x4161 => {
                let mut val = 0;
                if self.cpu_irq_pending {
                    val |= 0x40;
                }
                if self.ppu_state.irq_pending {
                    val |= 0x80;
                }

                val
            }
            0xfffa if self.redir_nmi => self.redir_nmi_lo,
            0xfffb if self.redir_nmi => self.redir_nmi_hi,
            0xfffe if self.redir_irq => self.redir_irq_lo,
            0xffff if self.redir_irq => self.redir_irq_hi,
            0x4800..=0x4fff => {
                let addr = addr - 0x4800;
                self.fpga_ram.read(addr + 0x1800)
            }
            0x5000..=0x5fff => {
                let bank = if self.fpga_ram_reg & 1 != 0 {
                    0x1000
                } else {
                    0x0000
                };
                self.fpga_ram.read((addr & 0xfff) | bank)
            }
            0x6000..0x8000 => {
                let bank = (addr >> 12) & 1;
                let fpga = if self.prg_mode & 0x80 == 0 {
                    (self.prg_ram_hi_regs[0] >> 6) & 3 == 3
                } else {
                    (self.prg_ram_hi_regs[bank as usize] >> 6) & 3 == 3
                };

                if fpga {
                    if self.prg_mode & 0x80 == 0 {
                        let addr = addr & 0x1fff;
                        self.fpga_ram.read(addr)
                    } else {
                        let addr = (addr & 0xfff)
                            | ((self.prg_ram_lo_regs[bank as usize] as u16 & 1) << 12);
                        self.fpga_ram.read(addr)
                    }
                } else {
                    self.prg.read(&self.cartridge, addr)
                }
            }
            0x8000.. => self.prg.read(&self.cartridge, addr),
            _ => 0,
        }
    }

    fn read_cpu(&mut self, addr: u16) -> u8 {
        match addr {
            0x4011 => {
                if self.cpu_irq_4011_ack {
                    self.cpu_irq_pending = false;
                    self.cpu_irq_enable = self.cpu_irq_ack_enable;
                }
            }
            0x4151 => {
                let mut val = 0;
                if self.ppu_state.irq_pending {
                    val |= 1;
                }
                if self.ppu_state.in_frame {
                    val |= 0x40;
                }
                if self.ppu_state.hblank() {
                    val |= 0x80;
                }

                self.ppu_state.irq_pending = false;
                return val;
            }
            0x415f => {
                let val = self.fpga_ram.read(self.fpga_reader_addr);
                self.fpga_reader_addr += self.fpga_reader_inc as u16;
                self.fpga_reader_addr &= 0x1fff;
                return val;
            }
            0x4280..=0x4286 => {
                tracing::error!("rainbow oam routine unsupported: {:04x}", addr);
            }
            0xfffa | 0xfffb => self.ppu_state.leave_frame(),
            _ => (),
        }
        self.peek_cpu(addr)
    }

    fn write_cpu(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000 => self.oam_state.ppu_ctrl(value),
            0x2003 => self.oam_state.oam_addr(value),
            0x2004 => self.oam_state.oam_data(value),
            0x4100 => {
                self.prg_mode = value;
                self.sync_prg()
            }
            0x4108..=0x410f => {
                self.prg_hi_regs[(addr & 7) as usize] = value;
                self.sync_prg()
            }
            0x4118..=0x411f => {
                self.prg_lo_regs[(addr & 7) as usize] = value;
                self.sync_prg()
            }
            0x4106..=0x4107 => {
                self.prg_ram_hi_regs[(addr & 1) as usize] = value;
                self.sync_prg()
            }
            0x4116..=0x4117 => {
                self.prg_ram_lo_regs[(addr & 1) as usize] = value;
                self.sync_prg()
            }
            0x4115 => self.fpga_ram_reg = value,
            0x4120 => {
                self.chr_mode = value;
                self.sync_chr();
            }
            0x4121 => self.bg_ext_hi = value,
            0x4126..=0x4129 => self.nt_bank_regs[(addr - 0x4126) as usize] = value,
            0x412a..=0x412d => self.nt_mode_regs[(addr - 0x412a) as usize] = value,
            0x412e => self.nt_bank_regs[4] = value,
            0x412f => self.nt_mode_regs[4] = (value & 0x3f) | 0x80,
            0x4124 => self.fill_tile = value,
            0x4125 => self.fill_attr = (value & 3) * 0b0101_0101,
            0x4130..=0x413f => {
                self.chr_hi_regs[(addr & 0xf) as usize] = value;
                self.sync_chr();
            }
            0x4140..=0x414f => {
                self.chr_lo_regs[(addr & 0xf) as usize] = value;
                self.sync_chr();
            }
            0x4150 => self.ppu_state.scanline_compare = value,
            0x4151 => self.ppu_irq_enabled = true,
            0x4152 => {
                self.ppu_irq_enabled = false;
                self.ppu_state.irq_pending = false;
            }
            0x4153 => self.ppu_state.irq_offset = value.max(1).min(170),
            0x4158 => self.cpu_irq_latch = (self.cpu_irq_latch & 0x00ff) | ((value as u16) << 8),
            0x4159 => self.cpu_irq_latch = (self.cpu_irq_latch & 0xff00) | (value as u16),
            0x415a => {
                self.cpu_irq_enable = value & 1 != 0;
                self.cpu_irq_ack_enable = value & 2 != 0;
                self.cpu_irq_4011_ack = value & 4 != 0;

                if self.cpu_irq_enable {
                    self.cpu_irq_counter = self.cpu_irq_latch;
                }
            }
            0x415b => {
                self.cpu_irq_pending = false;
                self.cpu_irq_enable = self.cpu_irq_ack_enable;
            }
            0x415c => {
                self.fpga_reader_addr =
                    (self.fpga_reader_addr & 0x00ff) | ((value as u16 & 0x1f) << 8)
            }
            0x415d => self.fpga_reader_addr = (self.fpga_reader_addr & 0xff00) | (value as u16),
            0x415e => self.fpga_reader_inc = value,
            0x415f => {
                self.fpga_ram.write(self.fpga_reader_addr, value);
                self.fpga_reader_addr += self.fpga_reader_inc as u16;
                self.fpga_reader_addr &= 0x1fff;
            }
            0x416b => {
                self.redir_nmi = value & 1 != 0;
                self.redir_irq = value & 2 != 0;
            }
            0x416c => self.redir_nmi_hi = value,
            0x416d => self.redir_nmi_lo = value,
            0x416e => self.redir_irq_hi = value,
            0x416f => self.redir_irq_lo = value,
            0x41a0 => self.pulse_a.volume(value),
            0x41a1 => self.pulse_a.freq_low(value),
            0x41a2 => self.pulse_a.freq_high(value),
            0x41a3 => self.pulse_b.volume(value),
            0x41a4 => self.pulse_b.freq_low(value),
            0x41a5 => self.pulse_b.freq_high(value),
            0x41a6 => self.sawtooth.accumulator_rate(value),
            0x41a7 => self.sawtooth.freq_low(value),
            0x41a8 => self.sawtooth.freq_high(value),
            0x41a9 => {
                self.audio_enable = value & 3 != 0;
                self.audio_4011_out = value & 4 != 0;
            }
            0x41aa => {
                let vol = (value & 0xf) as f32 / 15.0;
                self.master_volume = (i16::MAX as f32 * vol / 64.0) as i16;
            }
            0x4200..=0x423f => self.spr_ext_lo[(addr & 0x3f) as usize] = value,
            0x4240 => self.spr_ext_hi = value,
            0x4800..=0x4fff => {
                let addr = addr - 0x4800;
                self.fpga_ram.write(addr + 0x1800, value);
            }
            0x5000..=0x5fff => {
                let bank = if self.fpga_ram_reg & 1 != 0 {
                    0x1000
                } else {
                    0x0000
                };
                self.fpga_ram.write((addr & 0xfff) | bank, value);
            }
            0x6000..0x8000 => {
                let bank = (addr >> 12) & 1;
                let fpga = if self.prg_mode & 0x80 == 0 {
                    (self.prg_ram_hi_regs[0] >> 6) & 3 == 3
                } else {
                    (self.prg_ram_hi_regs[bank as usize] >> 6) & 3 == 3
                };

                if fpga {
                    if self.prg_mode & 0x80 == 0 {
                        let addr = addr & 0x1fff;
                        self.fpga_ram.write(addr, value);
                    } else {
                        let addr = (addr & 0xfff)
                            | ((self.prg_ram_lo_regs[bank as usize] as u16 & 1) << 12);
                        self.fpga_ram.write(addr, value);
                    }
                } else {
                    self.prg.write(addr, value);
                }
            }
            0x8000.. => self.prg.write(addr, value),
            _ => tracing::debug!("unsupported rainbow write reg: {addr:04x}:{value:02x}"),
        }
    }

    fn read_ppu(&self, addr: u16) -> u8 {
        if addr & 0x2000 != 0 {
            let nt_idx = ((addr >> 10) & 0x3) as usize;
            let mode = self.nt_mode_regs[nt_idx];
            let bank = self.nt_bank_regs[nt_idx];

            let read = self.ppu_state.read(None);
            if mode & 0x20 != 0 {
                match read {
                    Some(PpuRead::Attribute) => return self.fill_attr,
                    Some(PpuRead::Nametable) => return self.fill_tile,
                    _ => (),
                }
            }

            if mode & 0x1 != 0 {
                match read {
                    Some(PpuRead::ExtAttribute(attr)) => return attr,
                    _ => (),
                }
            }

            match (mode >> 6) & 3 {
                0 => 0, // internal NT
                1 => {
                    let chr_ram_limit = (self.cartridge.chr_ram_bytes >> 10) - 1;
                    let bank = bank as usize & chr_ram_limit;
                    let addr = (addr & 0x3ff) as usize;
                    self.chr
                        .read_in_bank(&self.cartridge, addr, 1, bank, BankKind::Ram)
                }
                2 => {
                    let bank = (bank as u16 & 3) * 0x400;
                    let addr = (addr & 0x3ff) | bank;
                    self.fpga_ram.read(addr)
                }
                3 => {
                    let chr_rom_limit = (self.cartridge.chr_rom.len() >> 10) - 1;
                    let bank = bank as usize & chr_rom_limit;
                    let addr = (addr & 0x3ff) as usize;
                    self.chr
                        .read_in_bank(&self.cartridge, addr, 1, bank, BankKind::Rom)
                }
                _ => unreachable!(),
            }
        } else {
            let oam = (self.chr_mode & 0x20 != 0).then_some(&self.oam_state);
            let ppu_read = self.ppu_state.read(oam);

            let read_from_bank = |addr, bank| {
                let addr = addr & 0xfff;
                match self.chr_mode >> 6 {
                    0 => {
                        let chr_rom_limit = (self.cartridge.chr_rom.len() >> 12) - 1;
                        let bank = bank as usize & chr_rom_limit;
                        self.chr.read_in_bank(
                            &self.cartridge,
                            addr as usize,
                            4,
                            bank,
                            BankKind::Rom,
                        )
                    }
                    1 => {
                        let chr_ram_limit = (self.cartridge.chr_ram_bytes >> 12) - 1;
                        let bank = bank as usize & chr_ram_limit;
                        self.chr.read_in_bank(
                            &self.cartridge,
                            addr as usize,
                            4,
                            bank,
                            BankKind::Ram,
                        )
                    }
                    _ => self.fpga_ram.read(addr),
                }
            };

            if let Some(PpuRead::ExtBg(bank)) = ppu_read {
                let bank = (bank as usize) | ((self.bg_ext_hi as usize & 0x1f) << 6);
                read_from_bank(addr, bank)
            } else if let Some(PpuRead::ExtSprite(sprite)) = ppu_read {
                let bank = self.spr_ext_lo[sprite as usize] as usize
                    | ((self.spr_ext_hi as usize & 0x07) << 8);
                read_from_bank(addr, bank)
            } else if self.chr_mode & 0x80 != 0 {
                self.fpga_ram.read(addr & 0xfff)
            } else {
                if self.chr_mode & 4 == 0 {
                    self.chr.read(&self.cartridge, addr)
                } else {
                    let bank_idx = (addr >> 9) as usize;
                    let hi = self.chr_hi_regs[bank_idx] as usize;
                    let lo = self.chr_lo_regs[bank_idx] as usize;
                    let bank = (hi << 8) | lo;
                    let addr = (addr as usize & 0x1ff) | ((bank & 1) << 9);
                    let bank = bank >> 1;

                    match self.chr_mode & 0x40 {
                        0 => {
                            let chr_rom_limit = (self.cartridge.chr_rom.len() >> 10) - 1;
                            let bank = bank as usize & chr_rom_limit;
                            self.chr.read_in_bank(
                                &self.cartridge,
                                addr as usize,
                                1,
                                bank,
                                BankKind::Rom,
                            )
                        }
                        _ => {
                            let chr_ram_limit = (self.cartridge.chr_ram_bytes >> 10) - 1;
                            let bank = bank as usize & chr_ram_limit;
                            self.chr.read_in_bank(
                                &self.cartridge,
                                addr as usize,
                                1,
                                bank,
                                BankKind::Ram,
                            )
                        }
                    }
                }
            }
        }
    }

    fn write_ppu(&mut self, addr: u16, value: u8) {
        if addr & 0x2000 != 0 {
            let nt_idx = ((addr >> 10) & 0x3) as usize;
            let mode = self.nt_mode_regs[nt_idx];
            let bank = self.nt_bank_regs[nt_idx];

            match (mode >> 6) & 3 {
                0 => (), // internal NT
                1 => {
                    let chr_ram_limit = (self.cartridge.chr_ram_bytes >> 10) - 1;
                    let bank = bank as usize & chr_ram_limit;
                    self.chr
                        .write_in_bank((addr & 0x3ff) as usize, 1, value, bank);
                }
                2 => {
                    let bank = (bank as u16 & 3) * 0x400;
                    let addr = (addr & 0x3ff) | bank;
                    self.fpga_ram.write(addr, value);
                }
                3 => (), // chr-rom,
                _ => unreachable!(),
            }
        } else if self.chr_mode & 0x80 != 0 {
            self.fpga_ram.write(addr & 0xfff, value)
        } else {
            if self.chr_mode & 4 == 0 {
                self.chr.write(addr, value)
            } else {
                let bank_idx = (addr >> 9) as usize;
                let hi = self.chr_hi_regs[bank_idx] as usize;
                let lo = self.chr_lo_regs[bank_idx] as usize;
                let bank = (hi << 8) | lo;
                let addr = (addr as usize & 0x1ff) | ((bank & 1) << 9);
                let bank = bank >> 1;

                if self.chr_mode & 0x40 != 0 {
                    let bank_limit = (self.cartridge.chr_ram_bytes >> 10) - 1;
                    self.chr.write_in_bank(addr, 1, value, bank & bank_limit)
                };
            }
        }
    }

    fn sync_prg(&mut self) {
        let prg_rom_limit = (self.cartridge.prg_rom.len() >> 10) - 1;
        let prg_ram_limit = (self.cartridge.prg_ram_bytes >> 10) - 1;

        let map_ram = |idx, size: usize| {
            let ram = self.prg_ram_hi_regs[idx] & 0x80 != 0;
            let hi = self.prg_ram_hi_regs[idx] & 0x7f;
            let lo = self.prg_ram_lo_regs[idx];

            let bank = ((hi as usize) << 8) | lo as usize;

            if ram {
                let bank_limit = prg_ram_limit >> size.trailing_zeros();
                (BankKind::Ram, bank & bank_limit)
            } else {
                let bank_limit = prg_rom_limit >> size.trailing_zeros();
                (BankKind::Rom, bank & bank_limit)
            }
        };

        if self.prg_mode & 0x80 == 0 {
            let (kind, bank) = map_ram(0, 8);
            self.prg.map(0x6000, 8, bank, kind);
        } else {
            let (kind, bank) = map_ram(0, 4);
            self.prg.map(0x6000, 4, bank, kind);
            let (kind, bank) = map_ram(1, 4);
            self.prg.map(0x7000, 4, bank, kind);
        }

        let map_rom = |idx, size: usize| {
            let ram = self.prg_hi_regs[idx] & 0x80 != 0;
            let hi = self.prg_hi_regs[idx] & 0x7f;
            let lo = self.prg_lo_regs[idx];

            let bank = ((hi as usize) << 8) | lo as usize;

            if ram {
                let bank_limit = prg_ram_limit >> size.trailing_zeros();
                (BankKind::Ram, bank & bank_limit)
            } else {
                let bank_limit = prg_rom_limit >> size.trailing_zeros();
                (BankKind::Rom, bank & bank_limit)
            }
        };

        match self.prg_mode & 0x7 {
            0 => {
                let (kind, bank) = map_rom(0, 32);
                self.prg.map(0x8000, 32, bank, kind);
            }
            1 => {
                let (kind, bank) = map_rom(0, 16);
                self.prg.map(0x8000, 16, bank, kind);
                let (kind, bank) = map_rom(4, 16);
                self.prg.map(0xc000, 16, bank, kind);
            }
            2 => {
                let (kind, bank) = map_rom(0, 16);
                self.prg.map(0x8000, 16, bank, kind);
                let (kind, bank) = map_rom(4, 8);
                self.prg.map(0xc000, 8, bank, kind);
                let (kind, bank) = map_rom(6, 8);
                self.prg.map(0xe000, 8, bank, kind);
            }
            3 => {
                for i in 0..4 {
                    let addr = 0x8000 + (i as u16 * 0x2000);
                    let (kind, bank) = map_rom(i * 2, 8);
                    self.prg.map(addr, 8, bank, kind);
                }
            }
            _ => {
                for i in 0..8 {
                    let addr = 0x8000 + (i as u16 * 0x1000);
                    let (kind, bank) = map_rom(i, 4);
                    self.prg.map(addr, 4, bank, kind);
                }
            }
        }
    }

    fn sync_chr(&mut self) {
        let ram = self.chr_mode & 0x40 != 0;
        let chr_rom_limit = (self.cartridge.chr_rom.len() >> 10) - 1;
        let chr_ram_limit = (self.cartridge.chr_ram_bytes >> 10) - 1;

        if self.chr_mode & 0x10 != 0 && !self.warn_window {
            tracing::error!("rainbow window split unsupported");
            self.warn_window = true;
        }

        let map_chr = |idx, size: usize| {
            let hi = self.chr_hi_regs[idx];
            let lo = self.chr_lo_regs[idx];

            let bank = ((hi as usize) << 8) | lo as usize;

            if ram {
                let bank_limit = chr_ram_limit >> size.trailing_zeros();
                (BankKind::Ram, bank & bank_limit)
            } else {
                let bank_limit = chr_rom_limit >> size.trailing_zeros();
                (BankKind::Rom, bank & bank_limit)
            }
        };
        match self.chr_mode & 7 {
            0 => {
                let (kind, bank) = map_chr(0, 8);
                self.chr.map(0x0000, 8, bank, kind);
            }
            1 => {
                let (kind, bank) = map_chr(0, 4);
                self.chr.map(0x0000, 4, bank, kind);
                let (kind, bank) = map_chr(1, 4);
                self.chr.map(0x1000, 4, bank, kind);
            }
            2 => {
                for i in 0..4 {
                    let (kind, bank) = map_chr(i, 2);
                    let addr = i as u16 * 0x800;
                    self.chr.map(addr, 2, bank, kind);
                }
            }
            3 => {
                for i in 0..8 {
                    let (kind, bank) = map_chr(i, 1);
                    let addr = i as u16 * 0x400;
                    self.chr.map(addr, 1, bank, kind);
                }
            }
            _ => {
                if !self.warn_512_banks {
                    tracing::error!("rainbow 512b chr banks unsupported");
                    self.warn_512_banks = true;
                }
                for i in 0..8 {
                    let (kind, bank) = map_chr(i, 1);
                    let addr = i as u16 * 0x400;
                    self.chr.map(addr, 1, bank >> 1, kind);
                }
            }
        }
    }
}

impl Mapper for Rainbow {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndAndMask(0x8000, 0xffff));
        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xe000, 0x6000, 0x7fff));

        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xf000, 0x5000, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xf000, 0x5000, 0xffff));

        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4100, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4100, 0xffff));

        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4200, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4200, 0xffff));

        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4200, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xff00, 0x4200, 0xffff));

        cpu.register_read(DeviceKind::Mapper, AndEqualsAndMask(0xf800, 0x4800, 0xffff));
        cpu.register_write(DeviceKind::Mapper, AndEqualsAndMask(0xf800, 0x4800, 0xffff));

        cpu.register_write(DeviceKind::Mapper, Address(0x2000));
        cpu.register_write(DeviceKind::Mapper, Address(0x2003));
        cpu.register_write(DeviceKind::Mapper, Address(0x2004));

        cpu.register_read(DeviceKind::Mapper, Address(0x4011));
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

    fn peek_ppu_fetch(&self, address: u16, _kind: PpuFetchKind) -> Nametable {
        if (address & 0x2000) != 0 {
            let nt_idx = ((address >> 10) & 0x3) as usize;
            if self.nt_mode_regs[nt_idx] == 0 {
                if self.nt_bank_regs[nt_idx] & 1 == 0 {
                    Nametable::InternalA
                } else {
                    Nametable::InternalB
                }
            } else {
                Nametable::External
            }
        } else {
            Nametable::External
        }
    }

    fn ppu_fetch(&mut self, address: u16, kind: PpuFetchKind) -> Nametable {
        if kind != PpuFetchKind::Idle {
            let nt_mode = if (address & 0x2000) != 0 {
                let nt_idx = ((address >> 10) & 0x3) as usize;
                Some(self.nt_mode_regs[nt_idx])
            } else {
                None
            };
            let was_irq = self.ppu_state.irq_pending;
            self.ppu_state
                .fetch(address, nt_mode, &self.fpga_ram, &mut self.oam_state);
            if self.ppu_irq_enabled && !was_irq && self.ppu_state.irq_pending {
                self.debug.event(crate::DebugEvent::MapperIrq);
            }
        }

        self.peek_ppu_fetch(address, kind)
    }

    fn tick(&mut self) {
        if self.cpu_irq_enable {
            self.cpu_irq_counter = self.cpu_irq_counter.saturating_sub(1);
            if self.cpu_irq_counter == 0 {
                self.cpu_irq_counter = self.cpu_irq_latch;
                self.cpu_irq_pending = true;
                self.debug.event(crate::DebugEvent::MapperIrq);
            }
        }
        self.ppu_state.tick();
        self.pulse_a.tick(FreqMode::X1);
        self.pulse_b.tick(FreqMode::X1);
        self.sawtooth.tick(FreqMode::X1);
    }

    fn get_irq(&mut self) -> bool {
        (self.ppu_irq_enabled && self.ppu_state.irq_pending)
            || (self.cpu_irq_enable && self.cpu_irq_pending)
    }

    fn get_sample(&self) -> Option<i16> {
        if self.audio_enable {
            let val = (self.pulse_a.sample() as i16
                + self.pulse_b.sample() as i16
                + self.sawtooth.sample() as i16)
                * self.master_volume;

            Some(val)
        } else {
            Some(0)
        }
    }

    fn save_wram(&self) -> Option<super::SaveWram> {
        if self.cartridge.battery {
            self.prg.save_wram()
        } else {
            None
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct ShadowOam {
    ppu_ctrl: u8,
    oam_addr: u8,
    #[cfg_attr(feature = "save-states", serde(with = "serde_arrays"))]
    oam: [u8; 256],
    line_oam: [u8; 8],
}

impl ShadowOam {
    fn new() -> Self {
        Self {
            ppu_ctrl: 0,
            oam_addr: 0,
            oam: [0; 256],
            line_oam: [0; 8],
        }
    }

    fn ppu_ctrl(&mut self, value: u8) {
        self.ppu_ctrl = value;
    }

    fn oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn oam_data(&mut self, value: u8) {
        self.oam[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn tall_sprite(&self) -> bool {
        self.ppu_ctrl & 0x20 != 0
    }

    fn eval(&mut self, scanline: u8) {
        let height = if self.tall_sprite() { 16 } else { 8 };
        let end = scanline + height;

        let mut sprites_on_line = 0;

        for (idx, s) in self.oam.chunks(4).enumerate() {
            if s[0] >= scanline && s[0] < end {
                self.line_oam[sprites_on_line] = idx as u8;
                sprites_on_line += 1;
                if sprites_on_line == 8 {
                    break;
                }
            }
        }
    }
}
