use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::bus::{AddressBus, BusKind, DeviceKind, RangeAndMask};
use crate::debug::{Debug, DebugEvent};
use crate::mapper::{Nametable, RcMapper};
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu_step::*;
use crate::region::{EmphMode, Region};
use crate::run_until::RunUntil;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FrameEnd {
    SetVblank,
    ClearVblank,
    Dot(u32, u32),
    Samples(usize),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PpuFetchKind {
    Idle,
    Read,
    Write,
}

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
#[derive(Debug, Copy, Clone, Default)]
pub struct PpuDebugState {
    pub tick: u64,
    pub scanline: u32,
    pub dot: u32,
    pub vblank: bool,
    pub nmi: bool,
    pub sprite_zero_hit: bool,
    pub registers: [u8; 8],
}

#[cfg(not(feature = "debugger"))]
#[derive(Debug, Copy, Clone, Default)]
pub struct PpuDebugState;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Ppu {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
    #[cfg_attr(feature = "save-states", save(skip))]
    mapper: RcMapper,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    nt_internal_a: FixedMemoryBlock<1>,
    nt_internal_b: FixedMemoryBlock<1>,
    #[cfg_attr(feature = "save-states", save(skip))]
    screen: Vec<u16>,

    current_tick: u64,
    last_status_read: u64,
    last_data_write: u64,
    pub frame: u32,
    regs: [u8; 8],
    vblank: bool,
    sprite_zero_hit: bool,
    sprite_overflow: bool,
    open_bus: OpenBus,

    write_latch: bool,

    data_read_buffer: u8,

    pub vram_addr: u16,
    pub vram_addr_temp: u16,
    vram_fine_x: u16,

    oam_addr: u8,
    pub(crate) oam_data: Vec<u8>,
    line_oam_data: [u8; 32],

    pub(crate) palette_data: [u8; 32],

    nametable_tile: u8,

    attribute_low: u8,
    attribute_high: u8,

    pattern_low: u8,
    pattern_high: u8,

    low_bg_shift: u16,
    high_bg_shift: u16,

    low_attr_shift: u16,
    high_attr_shift: u16,

    next_sprite_byte: u8,
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

    ppu_mask: DelayReg<4, u8>,

    #[cfg_attr(feature = "save-states", save(nested))]
    ppu_steps: PpuSteps,
    step: PpuStep,
}

impl Ppu {
    pub fn new(region: Region, mapper: RcMapper, debug: Rc<Debug>) -> Ppu {
        Ppu {
            region,
            mapper,
            debug,
            nt_internal_a: FixedMemoryBlock::new(),
            nt_internal_b: FixedMemoryBlock::new(),
            screen: vec![0x0f; 256 * 240],

            current_tick: 0,
            last_status_read: 0,
            last_data_write: 0,
            frame: 0,
            regs: [0; 8],
            vblank: false,
            sprite_zero_hit: false,
            sprite_overflow: false,
            open_bus: OpenBus::new(),

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

            next_sprite_byte: 0,
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

            ppu_mask: DelayReg::new(0),

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
        self.reset_delay = 0;
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
        let mut registers = self.regs;
        registers[2] = self.ppu_status();
        registers[3] = self.oam_addr;
        PpuDebugState {
            tick,
            scanline: self.step.scanline,
            dot: self.step.dot,
            vblank: self.vblank,
            nmi: self.nmi(),
            sprite_zero_hit: self.sprite_zero_hit,
            registers,
        }
    }
    #[cfg(not(feature = "debugger"))]
    pub fn debug_state(&self) -> PpuDebugState {
        PpuDebugState
    }

    #[cfg(feature = "debugger")]
    pub fn watch(&self, visitor: &mut crate::debug::WatchVisitor) {
        let mut ppu = visitor.group("PPU");
        ppu.value("Scanline", self.step.scanline);
        ppu.value("Dot", self.step.dot);
        ppu.value("VRAM Addr.", self.vram_addr);
        ppu.value("Vblank", self.vblank);
        ppu.value("NMI", self.nmi());
        ppu.value("Sprite Zero Hit", self.sprite_zero_hit);
        ppu.value("Sprite Overflow", self.sprite_overflow);
    }

    #[cfg(feature = "debugger")]
    pub fn peek(&self, address: u16) -> u8 {
        match address {
            0x2000 => self.open_bus.value(0x00),
            0x2001 => self.open_bus.value(0x00),
            0x2002 => self.ppu_status() | self.open_bus.value(0xe0),
            0x2003 => self.open_bus.value(0x00),
            0x2004 => self.oam_data[self.oam_addr as usize], //OAMDATA
            0x2005 => self.open_bus.value(0x00),
            0x2006 => self.open_bus.value(0x00),
            0x2007 => {
                //PPUDATA
                let addr = self.vram_addr;
                let (value, mask) = if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    let value = if self.is_grayscale() {
                        self.palette_data[addr as usize] & 0x30
                    } else {
                        self.palette_data[addr as usize]
                    };
                    (value, 0x3f)
                } else {
                    (self.data_read_buffer, 0xff)
                };

                value | self.open_bus.value(mask)
            }
            _ => unreachable!(),
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        match address {
            0x2000 => self.open_bus.value(0x00),
            0x2001 => self.open_bus.value(0x00),
            0x2002 => {
                //PPUSTATUS
                let status = self.ppu_status();
                self.write_latch = false;
                self.vblank = false;
                self.last_status_read = self.current_tick;
                self.open_bus.update(status, 0xe0);
                status | self.open_bus.value(0xe0)
            }
            0x2003 => self.open_bus.value(0x00), //OAMADDR
            0x2004 => {
                //OAMDATA
                let value = if self.is_rendering() && !self.in_vblank() {
                    self.next_sprite_byte
                } else {
                    self.oam_data[self.oam_addr as usize]
                };
                self.open_bus.update(value, 0xff);
                value
            }
            0x2005 => self.open_bus.value(0x00),
            0x2006 => self.open_bus.value(0x00),
            0x2007 => {
                //PPUDATA
                let addr = self.vram_addr;
                let (result, mask) = if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    let value = if self.is_grayscale() {
                        self.palette_data[addr as usize] & 0x30
                    } else {
                        self.palette_data[addr as usize]
                    };

                    (value, 0x3f)
                } else {
                    (self.data_read_buffer, 0xff)
                };
                self.open_bus.update(result, mask);
                self.data_read_buffer = self.ppu_read(self.vram_addr);
                if !self.in_vblank() && self.is_rendering() {
                    self.horz_increment();
                    self.vert_increment();
                } else {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_inc()) & 0x7fff;
                }

                result | self.open_bus.value(mask)
            }
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.open_bus.update(value, 0xff);
        match address {
            0x2000 => {
                //PPUCTRL
                if self.reset_delay != 0 {
                    return;
                }
                self.regs[0] = value;
                self.vram_addr_temp &= 0xf3ff;
                self.vram_addr_temp |= self.base_nametable();
            }
            0x2001 => {
                //PPUMASK
                if self.reset_delay != 0 {
                    return;
                }

                self.ppu_mask.update(value);
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
                    self.oam_addr = self.oam_addr.wrapping_add(4);
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
                } else {
                    self.vram_addr_temp &= 0x00ff;
                    self.vram_addr_temp |= ((value & 0x3f) as u16) << 8;
                }
                self.write_latch = !self.write_latch;
            }
            0x2007 => {
                //PPUDATA
                let rmw_write = self.current_tick - self.last_data_write <= 3;
                if rmw_write {
                    // Read-Modify-Write instructions to 2007 have odd behaviours: (tested by AccuracyCoin.nes)
                    //  1. only increment vram_addr once
                    //  2. If not writing to pallete, perform additional write to odd address
                    //
                    // Decrementing vram_addr here is to semi-emulate only incrementing once, this will be
                    // wrong if done during rendering
                    self.vram_addr = self.vram_addr.wrapping_sub(self.vram_inc()) & 0x7fff;
                }

                let addr = self.vram_addr;

                if addr & 0x3f00 == 0x3f00 {
                    let addr = if addr & 0x03 != 0 {
                        addr & 0x1f
                    } else {
                        addr & 0x0f
                    };
                    self.palette_data[addr as usize] = value;
                } else {
                    if rmw_write {
                        let rmw_addr = (self.vram_addr & 0xff00) | (value as u16);
                        self.ppu_write(rmw_addr, value);
                    }
                    self.ppu_write(self.vram_addr, value);
                }

                if !self.in_vblank() && self.is_rendering() {
                    self.horz_increment();
                    self.vert_increment();
                } else {
                    self.vram_addr = self.vram_addr.wrapping_add(self.vram_inc()) & 0x7fff;
                }
                self.last_data_write = self.current_tick;
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

    pub fn tick<U: RunUntil>(&mut self, frame_end: FrameEnd, until: &mut U) {
        if self.reset_delay != 0 {
            self.reset_delay -= 1;
        }

        until.add_dot();
        self.ppu_mask.tick();
        self.open_bus.tick();

        let mut step = self.ppu_steps.step();
        if frame_end == FrameEnd::Dot(step.scanline, step.dot) {
            until.add_frame();
            self.frame += 1;
        }

        self.debug.event(DebugEvent::Dot(step.scanline, step.dot));
        match step.state {
            Some(StateChange::SkippedTick) => {
                if self.frame % 2 == 1 && self.is_rendering() {
                    let skipped = self.ppu_steps.step();
                    step.scanline = 0;
                    step.dot = 0;
                    if frame_end == FrameEnd::Dot(step.scanline, step.dot) {
                        self.frame += 1;
                        until.add_frame();
                    }
                    self.debug.event(DebugEvent::Dot(step.scanline, step.dot));

                    // Need to assign this to ensure sprite reset still happens on dot 0 after a skip
                    step.sprite = skipped.sprite;
                }
            }
            Some(StateChange::SetVblank) => {
                self.vblank = self.last_status_read != self.current_tick;
                if frame_end == FrameEnd::SetVblank {
                    self.frame += 1;
                    until.add_frame();
                }
            }
            Some(StateChange::ClearVblank) => {
                self.sprite_zero_hit = false;
                self.sprite_overflow = false;
                self.vblank = false;
                if frame_end == FrameEnd::ClearVblank {
                    self.frame += 1;
                    until.add_frame();
                }
            }
            None => (),
        }

        if step.dot == 0 {
            until.add_scanline();
        }

        self.current_tick += 1;

        // Always reset sprite eval, even if rendering disabled
        if let Some(SpriteStep::Reset) = step.sprite {
            self.sprite_reset();
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
                Some(SpriteStep::Clear) => {
                    self.init_line_oam(step.dot / 2);
                }
                Some(SpriteStep::Eval) => {
                    self.sprite_eval(step.scanline, step.dot);
                }
                Some(SpriteStep::Read) => {
                    self.sprite_read();
                }
                Some(SpriteStep::Hblank) => {
                    self.sprite_eval(step.scanline, step.dot);

                    // Any on line is a performance optimization
                    self.sprite_any_on_line = false;
                }
                Some(SpriteStep::Fetch(n)) => {
                    self.oam_addr = 0;
                    match n {
                        1 => self.fetch_nametable(),
                        3 => self.fetch_attribute(),
                        5 => self.sprite_fetch(step.scanline, false),
                        7 => self.sprite_fetch(step.scanline, true),
                        _ => (),
                    }
                    match n {
                        0 => self.sprite_oam_read(0),
                        1 => self.sprite_oam_read(1),
                        2 => self.sprite_oam_read(2),
                        _ => self.sprite_oam_read(3),
                    }
                }
                Some(SpriteStep::BackgroundWait) => {
                    self.next_sprite_byte = self.line_oam_data[0];
                }
                _ => (),
            }
        }
        // outside of rending vram_addr is on bus (needed for mmc3)
        if !self.is_rendering() || self.in_vblank() {
            // only setting bus on odd cycles to acount for PPU mem access being 2 cycles long,
            // not sure on details when rendering disbaled / in vblank
            if step.dot & 1 == 1 {
                self.mapper
                    .ppu_fetch(self.vram_addr & 0x3fff, PpuFetchKind::Idle);
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

        let was_sprite_zero_hit = self.sprite_zero_hit;
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

        if !was_sprite_zero_hit && self.sprite_zero_hit {
            self.debug.event(DebugEvent::SpriteZero);
        }

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
        self.debug.event(DebugEvent::FetchSprite);
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
        self.sprite_oam_read(self.oam_addr & 3);
    }

    fn sprite_oam_read(&mut self, offset: u8) {
        self.next_sprite_byte = self.oam_data[((self.oam_addr & 0xfc) + offset) as usize];

        // OAM byte 2 bits 2-4 dont exist in hardware are read back as 0
        if offset == 2 {
            self.next_sprite_byte &= 0xe3;
        }
    }

    fn sprite_eval(&mut self, scanline: u32, dot: u32) {
        if self.sprite_read_loop {
            return;
        }

        struct Cursor {
            n: u8,
            m: u8,
        }

        impl Cursor {
            fn new(oam_addr: u8) -> Self {
                let n = oam_addr >> 2;
                let m = oam_addr & 3;
                Cursor { n, m }
            }

            fn advance_byte(&mut self) {
                self.m = self.m.wrapping_add(1);
                self.m &= 3;
                if self.m == 0 {
                    self.n = self.n.wrapping_add(1);
                }
            }

            fn advance_sprite(&mut self) {
                self.n = self.n.wrapping_add(1);
                self.m = 0;
            }

            // Everything I've read says that this should just be `self.advance_sprite()`,
            // but running sprite_evaluation_test.nes on hardware matches this behavior
            fn advance_sprite_x(&mut self) {
                if self.m == 3 {
                    self.advance_sprite();
                } else {
                    self.m = 0;
                }
            }

            fn advance_overflow(&mut self) {
                self.n = self.n.wrapping_add(1);
                self.m = self.m.wrapping_add(1);
                self.m &= 3;
            }

            fn is_at_end(&self) -> bool {
                self.n >= 64
            }

            fn oam_addr(&self) -> u8 {
                (self.n << 2) | (self.m & 3)
            }
        }

        let mut cursor = Cursor::new(self.oam_addr);

        if !self.block_oam_writes {
            self.line_oam_data[self.line_oam_index] = self.next_sprite_byte;
        }

        if dot == 66 {
            self.sprite_zero_on_next_line = false;
        }

        // Reading remaining 3 bytes of sprite
        if self.sprite_reads != 0 {
            // Sprite X on line check, normally doesn't matter unless OAMADDR is misaligned
            if self.sprite_reads == 1 && !self.sprite_on_line(self.next_sprite_byte, scanline) {
                cursor.advance_sprite_x();
            } else {
                cursor.advance_byte();
            }

            self.line_oam_index += 1;
            self.sprite_reads -= 1;
            if self.sprite_reads == 0 {
                self.found_sprites += 1;
                if self.found_sprites == 8 {
                    self.block_oam_writes = true;
                }
            }
        } else if self.sprite_on_line(self.next_sprite_byte, scanline) {
            // Sprite Y is on line, set up read of remaining bytes
            if dot == 66 {
                self.sprite_zero_on_next_line = true;
            }
            if self.found_sprites == 8 && !self.sprite_overflow {
                self.debug.event(DebugEvent::SpriteOverflow);
                self.sprite_overflow = true;
            }
            cursor.advance_byte();
            self.sprite_reads = 3;
            self.line_oam_index += 1;
        } else {
            // Sprite not on line, move to next sprite
            if self.found_sprites >= 8 {
                cursor.advance_overflow();
            } else {
                cursor.advance_sprite();
            }
        }

        if cursor.is_at_end() {
            self.sprite_read_loop = true;
            self.oam_addr = 0
        } else {
            self.oam_addr = cursor.oam_addr();
        }
    }

    fn init_line_oam(&mut self, addr: u32) {
        self.next_sprite_byte = 0xff;
        self.line_oam_data[addr as usize] = self.next_sprite_byte;
    }

    fn sprite_reset(&mut self) {
        self.sprite_render_index = 0;
        self.found_sprites = 0;
        self.sprite_reads = 0;
        self.line_oam_index = 0;
        self.sprite_read_loop = false;
        self.block_oam_writes = false;
        self.sprite_zero_on_line = self.sprite_zero_on_next_line;
        self.sprite_zero_on_next_line = false;
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
        self.debug.event(DebugEvent::FetchNt);
        let nt_addr = 0x2000 | (self.vram_addr & 0xfff);
        self.nametable_tile = self.ppu_read(nt_addr);
    }

    fn fetch_attribute(&mut self) {
        self.debug.event(DebugEvent::FetchAttr);
        let v = self.vram_addr;
        let at_addr = 0x23c0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attr = self.ppu_read(at_addr);

        let tile_idx = self.vram_addr & 0x3ff;

        let attr_row = tile_idx >> 5;
        let attr_col = tile_idx & 0x1f;
        let attr_bits = (attr_row & 0x2) | (attr_col >> 1 & 0x1);
        let palette = match attr_bits {
            0 => (attr >> 0) & 0x3,
            1 => (attr >> 2) & 0x3,
            2 => (attr >> 4) & 0x3,
            3 => (attr >> 6) & 0x3,
            _ => unreachable!(),
        };

        self.attribute_low = palette & 0x1;
        self.attribute_high = palette >> 1;
    }

    fn fetch_low_bg_pattern(&mut self) {
        self.debug.event(DebugEvent::FetchBg);
        let v = self.vram_addr;
        let tile_addr = ((v >> 12) & 0x07)
            | ((self.nametable_tile as u16) << 4)
            | self.background_pattern_table();
        self.pattern_low = self.ppu_read(tile_addr);
    }

    fn fetch_high_bg_pattern(&mut self) {
        self.debug.event(DebugEvent::FetchBg);
        let v = self.vram_addr;
        let tile_addr = ((v >> 12) & 0x07)
            | ((self.nametable_tile as u16) << 4)
            | self.background_pattern_table()
            | 0x08;
        self.pattern_high = self.ppu_read(tile_addr);
    }

    #[cfg(feature = "debugger")]
    pub fn ppu_peek(&self, address: u16) -> u8 {
        let bank = self
            .mapper
            .peek_ppu_fetch(address & 0x3fff, PpuFetchKind::Read);
        match bank {
            Nametable::InternalA => self.nt_internal_a.read(address),
            Nametable::InternalB => self.nt_internal_b.read(address),
            Nametable::External => self.mapper.peek(BusKind::Ppu, address & 0x3fff),
        }
    }

    fn ppu_read(&self, address: u16) -> u8 {
        self.debug.event(DebugEvent::PpuRead(address));
        let bank = self.mapper.ppu_fetch(address & 0x3fff, PpuFetchKind::Read);
        match bank {
            Nametable::InternalA => self.nt_internal_a.read(address),
            Nametable::InternalB => self.nt_internal_b.read(address),
            Nametable::External => self.mapper.read(BusKind::Ppu, address & 0x3fff),
        }
    }

    fn ppu_write(&mut self, address: u16, value: u8) {
        self.debug.event(DebugEvent::PpuWrite(address));
        let bank = self.mapper.ppu_fetch(address & 0x3fff, PpuFetchKind::Write);
        match bank {
            Nametable::InternalA => self.nt_internal_a.write(address, value),
            Nametable::InternalB => self.nt_internal_b.write(address, value),
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
        if self.regs[0] & 0x04 != 0 { 0x20 } else { 0x01 }
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
        self.ppu_mask.value() & 0x10 != 0
    }
    fn is_background_enabled(&self) -> bool {
        self.ppu_mask.value() & 0x08 != 0
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
        self.ppu_mask.value() & 0x18 != 0
    }

    fn ppu_status(&self) -> u8 {
        let mut value = 0;
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

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct DelayReg<
    const DELAY: usize,
    #[cfg(feature = "save-states")] T: Copy + Default + Serialize + DeserializeOwned,
    #[cfg(not(feature = "save-states"))] T: Copy + Default,
> {
    #[cfg_attr(feature = "save-states", serde(with = "serde_arrays"))]
    values: [T; DELAY],
}

impl<
    const DELAY: usize,
    #[cfg(feature = "save-states")] T: Copy + Default + Serialize + DeserializeOwned,
    #[cfg(not(feature = "save-states"))] T: Copy + Default,
> Default for DelayReg<DELAY, T>
{
    fn default() -> Self {
        Self {
            values: [T::default(); DELAY],
        }
    }
}

impl<
    const DELAY: usize,
    #[cfg(feature = "save-states")] T: Copy + Default + Serialize + DeserializeOwned,
    #[cfg(not(feature = "save-states"))] T: Copy + Default,
> DelayReg<DELAY, T>
{
    fn new(value: T) -> Self {
        const {
            if DELAY == 0 {
                panic!("DelayReg DELAY must be greater than 0")
            }
        }

        Self {
            values: [value; DELAY],
        }
    }

    fn tick(&mut self) {
        for i in 1..self.values.len() {
            self.values[i - 1] = self.values[i]
        }
    }

    fn value(&self) -> T {
        self.values[0]
    }

    fn update(&mut self, value: T) {
        self.values[DELAY - 1] = value;
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct OpenBus {
    value: u8,
    decay: [u32; 8],
}

impl OpenBus {
    fn new() -> Self {
        Self {
            value: 0,
            decay: [0; 8],
        }
    }

    fn tick(&mut self) {
        const DECAY_TICKS: u32 = 262 * 341 * 40;
        for idx in 0..8 {
            if self.decay[idx] >= DECAY_TICKS {
                let bit = 1 << idx;
                self.value &= !bit;
            } else {
                self.decay[idx] += 1;
            }
        }
    }

    fn update(&mut self, value: u8, mask: u8) {
        for idx in 0..8 {
            let bit = 1 << idx;
            if mask & bit != 0 {
                self.decay[idx] = 0;
                self.value &= !bit;
                self.value |= value & bit;
            }
        }
    }

    fn value(&self, mask: u8) -> u8 {
        self.value & !mask
    }
}
