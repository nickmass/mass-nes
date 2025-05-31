use crate::egui;
use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use nes::MachineState;

#[derive(Debug, Clone, Default)]
pub struct State(MachineState);

impl Deref for State {
    type Target = MachineState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Channels(Vec<nes::ChannelSamples>);

impl Deref for Channels {
    type Target = Vec<nes::ChannelSamples>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Channels {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct WatchItems(Vec<nes::WatchItem>);

impl Deref for WatchItems {
    type Target = Vec<nes::WatchItem>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WatchItems {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct SwapBuffer<T> {
    updated: Arc<AtomicBool>,
    data: Arc<Mutex<T>>,
}

impl<T: DerefMut> SwapBuffer<T> {
    fn new(data: T) -> Self {
        Self {
            updated: Arc::new(AtomicBool::new(false)),
            data: Arc::new(Mutex::new(data)),
        }
    }

    pub fn update<F: FnOnce(&mut <T as Deref>::Target)>(&mut self, func: F) {
        {
            let mut data = self.data.lock().unwrap();
            func(&mut data);
            self.updated.store(true, Ordering::Relaxed);
        }
    }

    pub fn attempt_swap(&self, other: &mut T) {
        if self.updated.load(Ordering::Relaxed) {
            if let Ok(mut data) = self.data.try_lock() {
                std::mem::swap(&mut *data, other);
                self.updated.store(false, Ordering::Relaxed);
            }
        }
    }
}

#[derive(Clone)]
pub struct DebugSwapState {
    pub now: Arc<AtomicU64>,
    pub cpu_mem: SwapBuffer<Vec<u8>>,
    pub ppu_mem: SwapBuffer<Vec<u8>>,
    pub pal_ram: SwapBuffer<Vec<u8>>,
    pub sprite_ram: SwapBuffer<Vec<u8>>,
    pub state: SwapBuffer<State>,
    pub breakpoint: Arc<AtomicBool>,
    pub events: SwapBuffer<Vec<(u8, u16)>>,
    pub frame: SwapBuffer<Vec<u16>>,
    pub channels: SwapBuffer<Channels>,
    pub watch_items: SwapBuffer<WatchItems>,
}

impl DebugSwapState {
    pub fn new() -> Self {
        Self {
            now: Arc::new(AtomicU64::new(0)),
            cpu_mem: SwapBuffer::new(vec![0; 0x10000]),
            ppu_mem: SwapBuffer::new(vec![0; 0x4000]),
            pal_ram: SwapBuffer::new(vec![0; 32]),
            sprite_ram: SwapBuffer::new(vec![0; 256]),
            state: SwapBuffer::new(State::default()),
            breakpoint: Arc::new(false.into()),
            events: SwapBuffer::new(vec![(0, 0); 312 * 341]),
            frame: SwapBuffer::new(vec![0; 256 * 240]),
            channels: SwapBuffer::new(Channels::default()),
            watch_items: SwapBuffer::new(WatchItems::default()),
        }
    }

    pub fn update_at(&self, time: u64) {
        self.now.store(time, Ordering::Relaxed);
    }

    pub fn set_breakpoint(&self) {
        self.breakpoint.store(true, Ordering::Relaxed)
    }

    pub fn on_breakpoint(&self) -> bool {
        self.breakpoint.load(Ordering::Relaxed)
    }
}

pub struct DebugUiState {
    swap: DebugSwapState,
    cpu_mem: Vec<u8>,
    ppu_mem: Vec<u8>,
    pal_ram: Vec<u8>,
    sprite_ram: Vec<u8>,
    state: State,
    palette: Palette,
    events: Vec<(u8, u16)>,
    frame: Vec<u16>,
    channels: Channels,
    watch_items: WatchItems,
}

impl DebugUiState {
    pub fn new(swap: DebugSwapState, palette: Palette) -> Self {
        Self {
            swap,
            cpu_mem: vec![0; 0x10000],
            ppu_mem: vec![0; 0x4000],
            pal_ram: vec![0; 32],
            sprite_ram: vec![0; 256],
            state: State::default(),
            palette,
            events: vec![(0, 0); 312 * 341],
            frame: vec![0; 256 * 240],
            channels: Channels::default(),
            watch_items: WatchItems::default(),
        }
    }

    pub fn now(&self) -> u64 {
        self.swap.now.load(Ordering::Relaxed)
    }

    pub fn breakpoint(&self) -> bool {
        self.swap.breakpoint.load(Ordering::Relaxed)
    }

    pub fn clear_breakpoint(&self) {
        self.swap.breakpoint.store(false, Ordering::Relaxed);
    }

    pub fn cpu_mem(&self) -> &[u8] {
        &self.cpu_mem
    }

    pub fn ppu_mem(&self) -> &[u8] {
        &self.ppu_mem
    }

    pub fn pal_ram(&self) -> &[u8] {
        &self.pal_ram
    }

    pub fn sprite_ram(&self) -> &[u8] {
        &self.sprite_ram
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn events(&self) -> &[(u8, u16)] {
        &self.events
    }

    pub fn frame(&self) -> &[u16] {
        &self.frame
    }

    pub fn ppu(&self) -> PpuView {
        PpuView(self)
    }

    pub fn channels(&self) -> &[nes::ChannelSamples] {
        &self.channels.0
    }

    pub fn watch_items(&self) -> &[nes::WatchItem] {
        &self.watch_items.0
    }

    pub fn swap(&mut self) {
        self.swap.cpu_mem.attempt_swap(&mut self.cpu_mem);
        self.swap.ppu_mem.attempt_swap(&mut self.ppu_mem);
        self.swap.pal_ram.attempt_swap(&mut self.pal_ram);
        self.swap.sprite_ram.attempt_swap(&mut self.sprite_ram);
        self.swap.state.attempt_swap(&mut self.state);
        self.swap.events.attempt_swap(&mut self.events);
        self.swap.frame.attempt_swap(&mut self.frame);
        self.swap.channels.attempt_swap(&mut self.channels);
        self.swap.watch_items.attempt_swap(&mut self.watch_items);
    }
}

pub struct Palette {
    bytes: Box<[u8]>,
}

impl Palette {
    pub fn new(bytes: impl Into<Box<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    pub fn lookup(&self, idx: u16) -> (u8, u8, u8) {
        let idx = idx as usize * 3;
        let r = self.bytes[idx + 0];
        let g = self.bytes[idx + 1];
        let b = self.bytes[idx + 2];
        (r, g, b)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ChrTable {
    Low,
    High,
}

impl ChrTable {
    fn base_addr(&self) -> u16 {
        match self {
            ChrTable::Low => 0x0000,
            ChrTable::High => 0x1000,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Nametable {
    One,
    Two,
    Three,
    Four,
}

impl Nametable {
    fn base_addr(&self) -> u16 {
        match self {
            Nametable::One => 0x2000,
            Nametable::Two => 0x2400,
            Nametable::Three => 0x2800,
            Nametable::Four => 0x2c00,
        }
    }

    fn attr_addr(&self) -> u16 {
        self.base_addr() + 0x3c0
    }
}

pub struct PpuView<'a>(&'a DebugUiState);

impl<'a> PpuView<'a> {
    pub fn regs(&self) -> &[u8] {
        &self.state.ppu.registers
    }

    pub fn pal_entry(&self, idx: u8) -> (u8, u8, u8) {
        let idx = if idx & 3 == 0 { 0 } else { idx };
        let idx = self.pal_ram()[idx as usize];
        self.palette().lookup(idx as u16)
    }

    pub fn pal_entry_color(&self, idx: u8) -> egui::Color32 {
        let (r, g, b) = self.pal_entry(idx);
        egui::Color32::from_rgb(r, g, b)
    }

    pub fn chr_tile(
        &self,
        palette: u8,
        chr_table: ChrTable,
        tile_idx: u8,
    ) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        let ppu_mem = self.ppu_mem();
        let base_addr = (((tile_idx as u16) << 4) | chr_table.base_addr()) as usize;

        let mut row = 0;
        let mut col = 0;

        let mut lo_plane = ppu_mem[base_addr];
        let mut hi_plane = ppu_mem[base_addr | 8];

        std::iter::from_fn(move || {
            if col == 8 {
                col = 0;
                row += 1;
                if row == 8 {
                    return None;
                }
                let lo_plane_idx = base_addr | row;
                let hi_plane_idx = base_addr | row | 8;

                lo_plane = ppu_mem[lo_plane_idx];
                hi_plane = ppu_mem[hi_plane_idx];
            }

            let pixel = ((lo_plane >> 7) & 0x1) | ((hi_plane >> 6) & 0x2);
            lo_plane <<= 1;
            hi_plane <<= 1;

            let pal_idx = (palette << 2) | pixel;

            col += 1;

            Some(self.pal_entry(pal_idx))
        })
    }

    pub fn nt_tile(&self, nt: Nametable, tile_idx: u16) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        let ppu_mem = self.ppu_mem();
        let addr = nt.base_addr() | tile_idx;
        let attr_addr = nt.attr_addr() | (tile_idx >> 4 & 0x38) | (tile_idx >> 2 & 7);
        let attr_row = tile_idx >> 5;
        let attr_col = tile_idx & 0x1f;
        let attr_bits = (attr_row & 0x2) | (attr_col >> 1 & 0x1);
        let tile_idx = ppu_mem[addr as usize];
        let attr = ppu_mem[attr_addr as usize];
        let palette = match attr_bits {
            0 => (attr >> 0) & 0x3,
            1 => (attr >> 2) & 0x3,
            2 => (attr >> 4) & 0x3,
            3 => (attr >> 6) & 0x3,
            _ => unreachable!(),
        };

        let chr_table = if self.regs()[0] & 0x10 != 0 {
            ChrTable::High
        } else {
            ChrTable::Low
        };

        self.chr_tile(palette, chr_table, tile_idx)
    }

    pub fn sprite(&self, sprite_idx: u8) -> Sprite {
        assert!(sprite_idx < 64);
        let oam_base = sprite_idx as usize * 4;
        let oam: [_; 4] = self.sprite_ram()[oam_base..oam_base + 4]
            .try_into()
            .unwrap();
        let tall = self.regs()[0] & 0x20 != 0;
        let chr_table = if tall {
            if oam[1] & 1 != 0 {
                ChrTable::High
            } else {
                ChrTable::Low
            }
        } else {
            if self.regs()[0] & 0x08 != 0 {
                ChrTable::High
            } else {
                ChrTable::Low
            }
        };

        let chr_idxs = if tall {
            &[oam[1] & 0xfe, oam[1] | 0x01][..]
        } else {
            &[oam[1]][..]
        };

        let palette = oam[2] & 0x3 | 0x4;
        let horz_flip = oam[2] & 0x40 != 0;
        let vert_flip = oam[2] & 0x80 != 0;

        let chr = chr_idxs
            .into_iter()
            .flat_map(|idx| self.chr_tile(palette, chr_table, *idx));

        let mut pixels = if tall {
            vec![(0, 0, 0); 8 * 16]
        } else {
            vec![(0, 0, 0); 8 * 8]
        };

        let (x_start, x_adj) = if horz_flip { (7, -1) } else { (0, 1) };
        let (y_start, y_adj) = match (tall, vert_flip) {
            (true, true) => (15, -1),
            (false, true) => (7, -1),
            _ => (0, 1),
        };

        let mut pixel_off_x = x_start;
        let mut pixel_off_y = y_start;

        let mut x_count = 0;

        for p in chr {
            if x_count == 8 {
                x_count = 0;
                pixel_off_y += y_adj;
                pixel_off_x = x_start;
            }

            let pixel_off = (pixel_off_y * 8 + pixel_off_x) as usize;
            pixels[pixel_off] = p;

            pixel_off_x += x_adj;
            x_count += 1;
        }

        Sprite { oam, tall, pixels }
    }
}

impl<'a> Deref for PpuView<'a> {
    type Target = DebugUiState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Sprite {
    pub oam: [u8; 4],
    pub tall: bool,
    pub pixels: Vec<(u8, u8, u8)>,
}

impl Sprite {
    pub fn y(&self) -> u8 {
        self.oam[0]
    }

    pub fn x(&self) -> u8 {
        self.oam[3]
    }

    pub fn hidden(&self) -> bool {
        self.y() >= 0xef
    }

    pub fn behind_bg(&self) -> bool {
        self.oam[2] & 0x20 != 0
    }
}
