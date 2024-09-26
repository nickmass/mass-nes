#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::bus::{AddressBus, BusKind, DeviceKind, RangeAndMask};
use crate::mapper::{Mapper, Nametable, RcMapper};
use crate::memory::MemoryBlock;
use crate::ppu_step::*;
use crate::region::{EmphMode, Region};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
struct SpriteData {
    active: u8,
    x: u8,
    attributes: u8,
    pattern_high: u8,
    pattern_low: u8,
}

impl Default for SpriteData {
    fn default() -> Self {
        SpriteData {
            active: 0,
            x: 0,
            attributes: 0,
            pattern_high: 0,
            pattern_low: 0,
        }
    }
}

#[allow(dead_code)]
#[cfg(feature = "debugger")]
#[derive(Debug, Copy, Clone)]
pub struct PpuDebugState {
    pub tick: u64,
    pub scanline: u32,
    pub dot: u32,
    pub vblank: bool,
    pub nmi: bool,
    pub sprite_zero_hit: bool,
}

#[cfg(not(feature = "debugger"))]
#[derive(Debug, Copy, Clone)]
pub struct PpuDebugState;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Ppu {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
    #[cfg_attr(feature = "save-states", save(skip))]
    mapper: RcMapper,
    nt_internal_a: MemoryBlock,
    nt_internal_b: MemoryBlock,
    #[cfg_attr(feature = "save-states", save(skip))]
    screen: Vec<u16>,

    current_tick: u64,
    last_status_read: u64,
    last_nmi_toggle: u64,
    pub frame: u32,
    regs: [u8; 8],
    vblank: bool,
    sprite_zero_hit: bool,
    sprite_overflow: bool,
    last_write: u8,

    write_latch: bool,

    data_read_buffer: u8,

    pub vram_addr: u16,
    pub vram_addr_temp: u16,
    vram_fine_x: u16,

    oam_addr: u8,
    oam_data: Vec<u8>,
    line_oam_data: [u8; 32],

    palette_data: [u8; 32],

    nametable_tile: u8,

    attribute_low: u8,
    attribute_high: u8,

    pattern_low: u8,
    pattern_high: u8,

    low_bg_shift: u16,
    high_bg_shift: u16,

    low_attr_shift: u16,
    high_attr_shift: u16,

    in_sprite_render: bool,
    next_sprite_byte: u8,
    sprite_n: u32,
    sprite_m: u32,
    sprite_read_loop: bool,
    block_oam_writes: bool,
    found_sprites: u32,
    sprite_reads: u32,
    line_oam_index: usize,
    sprite_zero_on_line: bool,
    sprite_zero_on_next_line: bool,
    sprite_any_on_line: bool,

    sprite_data: [SpriteData; 8],
    sprite_render_index: usize,

    reset_delay: u32,

    #[cfg_attr(feature = "save-states", save(skip))]
    ppu_steps: PpuSteps,
    step: PpuStep,
}

impl Ppu {
    pub fn new(region: Region, mapper: RcMapper) -> Ppu {
        Ppu {
            region,
            mapper,
            nt_internal_a: MemoryBlock::new(1),
            nt_internal_b: MemoryBlock::new(1),
            screen: vec![0x0f; 256 * 240],

            current_tick: 0,
            last_status_read: 0,
            last_nmi_toggle: 0,
            frame: 0,
            regs: [0; 8],
            vblank: false,
            sprite_zero_hit: false,
            sprite_overflow: false,
            last_write: 0,

            write_latch: false,

            data_read_buffer: 0,

            vram_addr: 0,
            vram_addr_temp: 0,
            vram_fine_x: 0,

            oam_addr: 0,
            oam_data: vec![0; 256],
            line_oam_data: [0; 32],

            palette_data: [0x0f; 32],

            nametable_tile: 0,

            attribute_low: 0,
            attribute_high: 0,

            pattern_low: 0,
            pattern_high: 0,

            low_bg_shift: 0,
            high_bg_shift: 0,

            low_attr_shift: 0,
            high_attr_shift: 0,

            in_sprite_render: false,
            next_sprite_byte: 0,
            sprite_n: 0,
            sprite_m: 0,
            sprite_read_loop: false,
            block_oam_writes: false,
            found_sprites: 0,
            sprite_reads: 0,
            line_oam_index: 0,
            sprite_zero_on_line: false,
            sprite_zero_on_next_line: false,
            sprite_any_on_line: false,

            sprite_data: [SpriteData::default(); 8],
            sprite_render_index: 0,

            reset_delay: 0,

            ppu_steps: generate_steps(region),
            step: PpuStep::default(),
        }
    }

    pub fn power(&mut self) {
        self.write(0x2000, 0);
        self.write(0x2001, 0);
        self.write(0x2002, 0xa0);
        self.write(0x2003, 0);
        self.write(0x2005, 0);
        self.write(0x2005, 0);
        self.write(0x2006, 0);
        self.write(0x2006, 0);

        self.data_read_buffer = 0;
        self.reset_delay = 29658 * 3;
    }

    pub fn reset(&mut self) {
        self.write(0x2000, 0);
        self.write(0x2001, 0);
        self.write(0x2005, 0);
        self.write(0x2005, 0);

        self.data_read_buffer = 0;
        self.reset_delay = 29658 * 3;
    }

    pub fn register(&self, cpu: &mut AddressBus) {
        cpu.register_read(DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x2007));
        cpu.register_write(DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x2007));
    }

    #[cfg(feature = "debugger")]
    pub fn debug_state(&self) -> PpuDebugState {
        let tick = self.current_tick;
        PpuDebugState {
            tick,
            scanline: self.step.scanline,
            dot: self.step.dot,
            vblank: self.vblank,
            nmi: self.nmi(),
            sprite_zero_hit: self.sprite_zero_hit,
        }
    }
    #[cfg(not(feature = "debugger"))]
    pub fn debug_state(&self) -> PpuDebugState {
        PpuDebugState
    }

    #[cfg(feature = "debugger")]
    pub fn peek(&self, address: u16) -> u8 {
        match address {
            0x2000 => self.last_write,
            0x2001 => self.last_write,
            0x2002 => self.ppu_status(),
            0x2003 => self.last_write,                       //OAMADDR
            0x2004 => self.oam_data[self.oam_addr as usize], //OAMDATA
            0x2005 => self.last_write,
            0x2006 => self.last_write,
            0x2007 => {
                //PPUDATA
                let addr = self.vram_addr;
                if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    if self.is_grayscale() {
                        self.palette_data[addr as usize] & 0x30
                    } else {
                        self.palette_data[addr as usize]
                    }
                } else {
                    self.data_read_buffer
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let value = match address {
            0x2000 => self.last_write,
            0x2001 => self.last_write,
            0x2002 => {
                //PPUSTATUS
                let status = self.ppu_status();
                self.write_latch = false;
                self.vblank = false;
                self.last_status_read = self.current_tick;
                status
            }
            0x2003 => self.last_write, //OAMADDR
            0x2004 => {
                //OAMDATA
                if self.is_rendering() && !self.in_vblank() {
                    self.next_sprite_byte
                } else {
                    self.oam_data[self.oam_addr as usize]
                }
            }
            0x2005 => self.last_write,
            0x2006 => self.last_write,
            0x2007 => {
                //PPUDATA
                let addr = self.vram_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    if self.is_grayscale() {
                        self.palette_data[addr as usize] & 0x30
                    } else {
                        self.palette_data[addr as usize]
                    }
                } else {
                    self.data_read_buffer
                };
                self.data_read_buffer = self.ppu_read(addr);
                if !self.in_vblank() && self.is_rendering() {
                    self.horz_increment();
                    self.vert_increment();
                } else {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_inc()) & 0x7fff;
                    let addr = self.vram_addr;
                    self.mapper.update_ppu_addr(addr);
                }

                result
            }
            _ => unreachable!(),
        };
        self.last_write = value;
        value
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.last_write = value;
        match address {
            0x2000 => {
                //PPUCTRL
                let was_nmi_enabled = self.is_nmi_enabled();
                if self.reset_delay != 0 {
                    return;
                }
                self.regs[0] = value;
                self.vram_addr_temp &= 0xf3ff;
                self.vram_addr_temp |= self.base_nametable();

                if was_nmi_enabled != self.is_nmi_enabled() {
                    self.last_nmi_toggle = self.current_tick;
                }
            }
            0x2001 => {
                //PPUMASK
                if self.reset_delay != 0 {
                    return;
                }
                self.regs[1] = value;
            }
            0x2002 => {
                self.regs[2] = value;
            }
            0x2003 => {
                //OAMADDR
                self.oam_addr = value;
            }
            0x2004 => {
                //OAMDATA
                if !self.in_vblank() && self.is_rendering() {
                    self.sprite_n += 1;
                    if self.sprite_n == 64 {
                        self.sprite_n = 0;
                    }
                } else {
                    // OAM byte 2 bits 2-4 dont exist in hardware are read back as 0
                    if self.oam_addr & 3 == 2 {
                        self.oam_data[self.oam_addr as usize] = value & 0xe3;
                    } else {
                        self.oam_data[self.oam_addr as usize] = value;
                    }
                    self.oam_addr = self.oam_addr.wrapping_add(1);
                }
            }
            0x2005 => {
                //PPUSCROLL
                if self.reset_delay != 0 {
                    return;
                }
                if self.write_latch {
                    let value = value as u16;
                    self.vram_addr_temp &= 0x0c1f;
                    self.vram_addr_temp |= (value & 0xf8) << 2;
                    self.vram_addr_temp |= (value & 0x07) << 12;
                } else {
                    self.vram_addr_temp &= 0x7fe0;
                    self.vram_addr_temp |= (value >> 3) as u16;
                    self.vram_fine_x = (value & 0x07) as u16;
                }
                self.write_latch = !self.write_latch;
            }
            0x2006 => {
                //PPUADDR
                if self.reset_delay != 0 {
                    return;
                }
                if self.write_latch {
                    self.vram_addr_temp &= 0x7f00;
                    self.vram_addr_temp |= value as u16;
                    self.vram_addr = self.vram_addr_temp;
                    let addr = self.vram_addr;
                    self.mapper.update_ppu_addr(addr);
                } else {
                    self.vram_addr_temp &= 0x00ff;
                    self.vram_addr_temp |= ((value & 0x3f) as u16) << 8;
                }
                self.write_latch = !self.write_latch;
            }
            0x2007 => {
                //PPUDATA
                let addr = self.vram_addr;
                if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    self.palette_data[addr as usize] = value;
                } else {
                    self.ppu_write(addr & 0x3fff, value);
                }
                if !self.in_vblank() && self.is_rendering() {
                    self.horz_increment();
                    self.vert_increment();
                } else {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_inc()) & 0x7fff;
                    let addr = self.vram_addr;
                    self.mapper.update_ppu_addr(addr);
                }
            }
            _ => {
                tracing::error!("unreachable ppu register: {:04X}", address);
                unreachable!()
            }
        }
    }

    pub fn nmi(&self) -> bool {
        self.vblank && self.is_nmi_enabled()
    }

    pub fn tick(&mut self) {
        if self.reset_delay != 0 {
            self.reset_delay -= 1;
        }

        let mut step = self.ppu_steps.step();

        match step.state {
            Some(StateChange::SkippedTick) => {
                if self.frame % 2 == 1 && self.is_rendering() {
                    let _ = self.ppu_steps.step();
                    step.scanline = 0;
                    step.dot = 0;
                }
            }
            Some(StateChange::SetVblank) => {
                self.vblank = self.last_status_read != self.current_tick;
            }
            Some(StateChange::ClearVblank) => {
                self.sprite_zero_hit = false;
                self.sprite_overflow = false;
                self.vblank = false;
                self.frame += 1;
            }
            None => (),
        }

        self.current_tick += 1;

        // Always reset sprite eval, even if rendering disabled
        if let Some(SpriteStep::Reset) = step.sprite {
            self.sprite_render_index = 0;
            self.sprite_n = 0;
            self.sprite_m = 0;
            self.found_sprites = 0;
            self.sprite_reads = 0;
            self.line_oam_index = 0;
            self.sprite_read_loop = false;
            self.block_oam_writes = false;
            self.sprite_zero_on_line = self.sprite_zero_on_next_line;
            self.sprite_zero_on_next_line = false;
        }

        if self.is_rendering() {
            match step.background {
                Some(BackgroundStep::VertReset) => {
                    self.vert_reset();
                }
                Some(BackgroundStep::HorzReset) => {
                    self.horz_reset();
                }
                Some(BackgroundStep::VertIncrement) => {
                    self.horz_increment();
                    self.vert_increment();
                }
                Some(BackgroundStep::HorzIncrement) => {
                    self.load_bg_shifters();
                    self.horz_increment();
                }
                Some(BackgroundStep::ShiftedHorzIncrement) => {
                    self.low_bg_shift <<= 8;
                    self.high_bg_shift <<= 8;
                    self.low_attr_shift <<= 8;
                    self.high_attr_shift <<= 8;
                    self.load_bg_shifters();
                    self.horz_increment();
                }
                Some(BackgroundStep::Nametable) => {
                    self.fetch_nametable();
                }
                Some(BackgroundStep::Attribute) => {
                    self.fetch_attribute();
                }
                Some(BackgroundStep::LowPattern) => {
                    self.fetch_low_bg_pattern();
                }
                Some(BackgroundStep::HighPattern) => {
                    self.fetch_high_bg_pattern();
                }
                None => (),
            }

            match step.sprite {
                Some(SpriteStep::Reset) => {
                    self.in_sprite_render = false;
                    self.init_line_oam(0);
                }
                Some(SpriteStep::Clear) => {
                    self.in_sprite_render = false;
                    self.init_line_oam(step.dot / 2);
                }
                Some(SpriteStep::Eval) => {
                    self.sprite_eval(step.scanline);
                }
                Some(SpriteStep::Read) => {
                    self.in_sprite_render = false;
                    self.sprite_read();
                }
                Some(SpriteStep::Hblank) => {
                    self.sprite_n = 0;
                    self.sprite_eval(step.scanline);
                    self.sprite_any_on_line = false;
                }
                Some(SpriteStep::Fetch(0)) => self.sprite_oam_read(0),
                Some(SpriteStep::Fetch(1)) => {
                    self.sprite_oam_read(1);
                    self.fetch_nametable();
                }
                Some(SpriteStep::Fetch(2)) => self.sprite_oam_read(2),
                Some(SpriteStep::Fetch(3)) => {
                    self.sprite_oam_read(3);
                    self.fetch_attribute();
                }
                Some(SpriteStep::Fetch(4)) => self.sprite_oam_read(3),
                Some(SpriteStep::Fetch(5)) => {
                    self.sprite_oam_read(3);
                    self.sprite_fetch(step.scanline, false);
                }
                Some(SpriteStep::Fetch(6)) => self.sprite_oam_read(3),
                Some(SpriteStep::Fetch(7)) => {
                    self.sprite_oam_read(3);
                    self.sprite_fetch(step.scanline, true);
                }
                Some(SpriteStep::BackgroundWait) => {
                    self.next_sprite_byte = self.line_oam_data[0];
                }
                None => (),
                _ => unreachable!(),
            }
        }

        if step.scanline < self.region.vblank_line() && step.dot < 256 {
            self.render(step.dot, step.scanline);
        }

        self.step = step;
    }

    fn render(&mut self, dot: u32, scanline: u32) {
        let fine_x = self.vram_fine_x;
        let color = (((self.low_bg_shift >> (15 - fine_x)) & 0x1)
            | ((self.high_bg_shift >> (14 - fine_x)) & 0x2)) as u16;
        let attr = (((self.low_attr_shift >> (15 - fine_x)) & 0x1)
            | ((self.high_attr_shift >> (14 - fine_x)) & 0x2)) as u16;

        let attr = if color == 0 { 0 } else { attr << 2 };

        let palette = color | attr;
        let mut sprite_zero = false;
        let mut sprite_pixel = 0;
        let mut behind_bg = false;
        let left_sprites = self.is_left_sprites();
        if self.is_sprites_enabled() && self.sprite_any_on_line {
            for (idx, sprite) in self.sprite_data.iter_mut().enumerate() {
                if sprite.x == 0 {
                    sprite.active = 1;
                }
                if sprite.active > 0 && sprite.active <= 8 {
                    let attr = sprite.attributes;
                    let high = sprite.pattern_high;
                    let low = sprite.pattern_low;
                    let flip_horz = attr & 0x40 != 0;
                    let pal = (attr & 0x3) << 2;

                    let pal_bit = if flip_horz { 0x1 } else { 0x80 };
                    let mut color = if high & pal_bit != 0 { 2 } else { 0 }
                        | if low & pal_bit != 0 { 1 } else { 0 };

                    if !left_sprites && dot < 8 {
                        color = 0;
                    }

                    if color != 0 && sprite_pixel == 0 {
                        sprite_zero = idx == 0 && self.sprite_zero_on_line && dot < 255;
                        sprite_pixel = color | pal;
                        behind_bg = attr & 0x20 != 0;
                    }

                    sprite.active += 1;

                    if flip_horz {
                        sprite.pattern_high >>= 1;
                        sprite.pattern_low >>= 1;
                    } else {
                        sprite.pattern_high <<= 1;
                        sprite.pattern_low <<= 1;
                    }
                }

                if sprite.active == 0 && sprite.x != 0 {
                    sprite.x -= 1;
                }
            }
        }

        let bg_colored =
            color != 0 && (dot > 7 || self.is_left_background()) && self.is_background_enabled();
        let sprite_colored = sprite_pixel != 0;

        let pixel = match (bg_colored, sprite_colored, behind_bg) {
            (false, false, _) => 0x3f00,
            (false, true, _) => 0x3f10 | sprite_pixel as u16,
            (true, false, _) => 0x3f00 | palette as u16,
            (true, true, false) => {
                if sprite_zero {
                    self.sprite_zero_hit = true;
                }
                0x3f10 | sprite_pixel as u16
            }
            (true, true, true) => {
                if sprite_zero {
                    self.sprite_zero_hit = true;
                }
                0x3f00 | palette as u16
            }
        };

        let pixel = if !self.is_rendering() && self.vram_addr & 0x3f00 == 0x3f00 {
            self.vram_addr & 0x3f1f
        } else {
            pixel
        };
        let addr = if pixel & 0x03 != 0 {
            pixel & 0x1f
        } else {
            pixel & 0x0f
        };
        let mut pixel_result = self.palette_data[addr as usize];

        if self.is_grayscale() {
            pixel_result &= 0x30;
        }

        /*
        if system.debug.color {
            pixel_result = 0x14
        }
        */

        self.screen[((scanline * 256) + dot) as usize] = pixel_result as u16 | self.emph_bits();

        self.low_attr_shift <<= 1;
        self.high_attr_shift <<= 1;
        self.low_bg_shift <<= 1;
        self.high_bg_shift <<= 1;
    }

    fn sprite_on_line(&self, sprite_y: u8, scanline: u32) -> bool {
        if sprite_y > 239 {
            return false;
        }
        if self.is_tall_sprites() {
            (sprite_y as u32) + 16 > scanline && (sprite_y as u32) <= scanline
        } else {
            (sprite_y as u32) + 8 > scanline && (sprite_y as u32) <= scanline
        }
    }

    fn sprite_fetch(&mut self, scanline: u32, high: bool) {
        let index = self.sprite_render_index;
        let sprite_y = self.line_oam_data[index * 4];
        let sprite_tile = self.line_oam_data[(index * 4) + 1] as u16;
        let sprite_attr = self.line_oam_data[(index * 4) + 2];
        let sprite_x = self.line_oam_data[(index * 4) + 3];

        let flip_vert = sprite_attr & 0x80 != 0;
        let sprite_height = if self.is_tall_sprites() { 16 } else { 8 };
        let line = if scanline >= sprite_y as u32 && scanline - (sprite_y as u32) < sprite_height {
            (scanline - sprite_y as u32) as u16
        } else {
            0
        };
        let tile_addr = if self.is_tall_sprites() {
            let bottom_half = line >= 8;
            let line = if bottom_half { line - 8 } else { line };
            let line = if flip_vert { 7 - line } else { line };
            let pattern_table = (sprite_tile as u16 & 1) << 12;
            let sprite_tile = sprite_tile & 0xfe;

            match (flip_vert, bottom_half) {
                (true, true) | (false, false) => ((sprite_tile << 4) | pattern_table) + line,
                (false, true) | (true, false) => (((sprite_tile + 1) << 4) | pattern_table) + line,
            }
        } else {
            let line = if flip_vert { 7 - line } else { line };
            ((sprite_tile << 4) | self.sprite_pattern_table()) + line
        };

        let pattern_addr = if high { tile_addr | 0x08 } else { tile_addr };
        let pattern_byte = self.ppu_read(pattern_addr);
        let sprite_on_line = self.sprite_on_line(sprite_y, scanline);
        self.sprite_any_on_line |= sprite_on_line;

        let sprite = &mut self.sprite_data[index];
        sprite.x = sprite_x;
        sprite.attributes = sprite_attr;
        sprite.active = 0;
        if high {
            sprite.pattern_high = pattern_byte;
            if !sprite_on_line {
                sprite.pattern_high = 0;
            }
            self.sprite_render_index += 1;
        } else {
            sprite.pattern_low = pattern_byte;
            if !sprite_on_line {
                sprite.pattern_low = 0;
            }
        }
    }

    fn sprite_read(&mut self) {
        self.sprite_oam_read(self.sprite_m);
    }

    fn sprite_oam_read(&mut self, offset: u32) {
        self.next_sprite_byte = self.oam_data[((self.sprite_n * 4) + offset) as usize];

        // OAM byte 2 bits 2-4 dont exist in hardware are read back as 0
        if offset == 2 {
            self.next_sprite_byte &= 0xe3;
        }
    }

    fn sprite_eval(&mut self, scanline: u32) {
        if self.sprite_read_loop {
            return;
        }

        if !self.block_oam_writes {
            self.line_oam_data[self.line_oam_index] = self.next_sprite_byte;
        }
        if self.found_sprites == 8 {
            if self.sprite_reads != 0 {
                self.sprite_m += 1;
                self.sprite_m &= 3;
                if self.sprite_m == 0 {
                    self.sprite_n += 1;
                    if self.sprite_n == 64 {
                        self.sprite_read_loop = true;
                        self.sprite_n = 0;
                        self.sprite_m = 0;
                    }
                }
                self.sprite_reads -= 1;
            } else if self.sprite_on_line(self.next_sprite_byte, scanline) {
                self.sprite_overflow = true;
                self.sprite_m += 1;
                self.sprite_m &= 3;
                self.sprite_reads = 3;
            } else {
                self.sprite_n += 1;
                self.sprite_m += 1;
                self.sprite_m &= 3;
                if self.sprite_n == 64 {
                    self.sprite_read_loop = true;
                    self.sprite_n = 0;
                }
            }
        } else {
            //Less then 8 sprites found
            if self.sprite_reads != 0 {
                self.sprite_m += 1;
                self.sprite_m &= 3;
                self.line_oam_index += 1;
                self.sprite_reads -= 1;
                if self.sprite_reads == 0 {
                    self.found_sprites += 1;
                }
            } else if self.sprite_on_line(self.next_sprite_byte, scanline) {
                if self.sprite_n == 0 {
                    self.sprite_zero_on_next_line = true;
                }
                self.sprite_m += 1;
                self.sprite_reads = 3;
                self.line_oam_index += 1;
            }
            if self.sprite_reads == 0 {
                self.sprite_n += 1;
                self.sprite_m = 0;
                if self.sprite_n == 64 {
                    self.sprite_read_loop = true;
                    self.sprite_n = 0;
                } else if self.found_sprites == 8 {
                    self.block_oam_writes = true;
                }
            }
        }
    }

    fn init_line_oam(&mut self, addr: u32) {
        self.in_sprite_render = true;
        self.next_sprite_byte = 0xff;
        self.line_oam_data[addr as usize] = self.next_sprite_byte;
    }

    fn horz_increment(&mut self) {
        let mut addr = self.vram_addr;
        if addr & 0x001f == 0x1f {
            addr &= !0x001f;
            addr ^= 0x0400;
        } else {
            addr += 1;
        }
        self.vram_addr = addr;
    }

    fn vert_increment(&mut self) {
        let mut addr = self.vram_addr;
        if (addr & 0x7000) != 0x7000 {
            addr += 0x1000;
        } else {
            addr &= !0x7000;
            let mut y = (addr & 0x03e0) >> 5;
            if y == 29 {
                y = 0;
                addr ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }

            addr = (addr & !0x03e0) | (y << 5);
        }
        self.vram_addr = addr;
    }

    fn horz_reset(&mut self) {
        let mut addr = self.vram_addr;
        let addr_t = self.vram_addr_temp;

        addr &= 0xfbe0;
        addr |= addr_t & 0x041f;
        self.vram_addr = addr;
    }

    fn vert_reset(&mut self) {
        let mut addr = self.vram_addr;
        let addr_t = self.vram_addr_temp;

        addr &= 0x841f;
        addr |= addr_t & 0x7be0;
        self.vram_addr = addr;
    }

    fn load_bg_shifters(&mut self) {
        self.low_bg_shift &= 0xff00;
        self.low_bg_shift |= self.pattern_low as u16;
        self.high_bg_shift &= 0xff00;
        self.high_bg_shift |= self.pattern_high as u16;

        self.low_attr_shift &= 0xff00;
        self.low_attr_shift |= ((self.attribute_low & 1) * 0xff) as u16;
        self.high_attr_shift &= 0xff00;
        self.high_attr_shift |= ((self.attribute_high & 1) * 0xff) as u16;
    }

    fn fetch_nametable(&mut self) {
        let nt_addr = 0x2000 | (self.vram_addr & 0xfff);
        self.nametable_tile = self.ppu_read(nt_addr);
    }

    fn fetch_attribute(&mut self) {
        let v = self.vram_addr;
        let at_addr = 0x23c0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attr = self.ppu_read(at_addr);

        let tile_num = self.vram_addr & 0x3ff;
        let tile_x = tile_num % 32;
        let tile_y = tile_num / 32;

        let attr_quad = ((tile_y >> 1) & 1, (tile_x >> 1) & 1);
        match attr_quad {
            (0, 0) => {
                self.attribute_low = (attr >> 0) & 1;
                self.attribute_high = (attr >> 1) & 1;
            }
            (0, 1) => {
                self.attribute_low = (attr >> 2) & 1;
                self.attribute_high = (attr >> 3) & 1;
            }
            (1, 0) => {
                self.attribute_low = (attr >> 4) & 1;
                self.attribute_high = (attr >> 5) & 1;
            }
            (1, 1) => {
                self.attribute_low = (attr >> 6) & 1;
                self.attribute_high = (attr >> 7) & 1;
            }
            _ => unreachable!(),
        }
    }

    fn fetch_low_bg_pattern(&mut self) {
        let v = self.vram_addr;
        let tile_addr = ((v >> 12) & 0x07)
            | ((self.nametable_tile as u16) << 4)
            | self.background_pattern_table();
        self.pattern_low = self.ppu_read(tile_addr);
    }

    fn fetch_high_bg_pattern(&mut self) {
        let v = self.vram_addr;
        let tile_addr = ((v >> 12) & 0x07)
            | ((self.nametable_tile as u16) << 4)
            | self.background_pattern_table()
            | 0x08;
        self.pattern_high = self.ppu_read(tile_addr);
    }

    fn ppu_read(&self, address: u16) -> u8 {
        let bank = self.mapper.ppu_fetch(address & 0x3fff);
        match bank {
            Nametable::InternalA => self.nt_internal_a.read(address & 0x3ff),
            Nametable::InternalB => self.nt_internal_b.read(address & 0x3ff),
            Nametable::External => self.mapper.read(BusKind::Ppu, address & 0x3fff),
        }
    }

    fn ppu_write(&self, address: u16, value: u8) {
        let bank = self.mapper.ppu_fetch(address & 0x3fff);
        match bank {
            Nametable::InternalA => self.nt_internal_a.write(address & 0x3ff, value),
            Nametable::InternalB => self.nt_internal_b.write(address & 0x3ff, value),
            Nametable::External => self.mapper.write(BusKind::Ppu, address & 0x3fff, value),
        }
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }
    pub fn screen(&self) -> &[u16] {
        self.screen.as_ref()
    }

    fn is_nmi_enabled(&self) -> bool {
        self.regs[0] & 0x80 != 0
    }

    fn is_tall_sprites(&self) -> bool {
        self.regs[0] & 0x20 != 0
    }

    fn background_pattern_table(&self) -> u16 {
        if self.regs[0] & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    fn sprite_pattern_table(&self) -> u16 {
        if self.regs[0] & 0x08 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    fn vram_inc(&self) -> u16 {
        if self.regs[0] & 0x04 != 0 {
            0x20
        } else {
            0x01
        }
    }

    fn base_nametable(&self) -> u16 {
        (self.regs[0] as u16 & 3) << 10
    }

    fn emph_bits(&self) -> u16 {
        let mut val = 0;
        match self.region.emph_bits() {
            EmphMode::Bgr => {
                if self.is_red_emph() {
                    val |= 0x40;
                }
                if self.is_green_emph() {
                    val |= 0x80;
                }
                if self.is_blue_emph() {
                    val |= 0x100;
                }
            }
            EmphMode::Brg => {
                if self.is_green_emph() {
                    val |= 0x40;
                }
                if self.is_red_emph() {
                    val |= 0x80;
                }
                if self.is_blue_emph() {
                    val |= 0x100;
                }
            }
        }
        val
    }
    fn is_blue_emph(&self) -> bool {
        self.regs[1] & 0x80 != 0
    }
    fn is_green_emph(&self) -> bool {
        self.regs[1] & 0x40 != 0
    }
    fn is_red_emph(&self) -> bool {
        self.regs[1] & 0x20 != 0
    }
    fn is_sprites_enabled(&self) -> bool {
        self.regs[1] & 0x10 != 0
    }
    fn is_background_enabled(&self) -> bool {
        self.regs[1] & 0x08 != 0
    }
    fn is_left_sprites(&self) -> bool {
        self.regs[1] & 0x04 != 0
    }
    fn is_left_background(&self) -> bool {
        self.regs[1] & 0x02 != 0
    }
    fn is_grayscale(&self) -> bool {
        self.regs[1] & 0x01 != 0
    }
    fn is_rendering(&self) -> bool {
        self.is_sprites_enabled() || self.is_background_enabled()
    }

    fn ppu_status(&self) -> u8 {
        let mut value = self.last_write & 0x1f;
        if self.sprite_overflow {
            value |= 0x20;
        }
        if self.sprite_zero_hit {
            value |= 0x40;
        }
        if self.vblank {
            value |= 0x80;
        }
        value
    }

    fn in_vblank(&self) -> bool {
        self.step.scanline >= self.region.vblank_line()
            && self.step.scanline < self.region.prerender_line()
    }
}
