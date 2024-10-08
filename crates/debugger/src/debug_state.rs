use crate::egui;
use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

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
            let mut data = self.data.lock().unwrap();
            std::mem::swap(&mut *data, other);
            self.updated.store(false, Ordering::Relaxed);
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
}

impl DebugSwapState {
    pub fn new() -> Self {
        Self {
            now: Arc::new(AtomicU64::new(0)),
            cpu_mem: SwapBuffer::new(vec![0; 0x10000]),
            ppu_mem: SwapBuffer::new(vec![0; 0x4000]),
            pal_ram: SwapBuffer::new(vec![0; 32]),
            sprite_ram: SwapBuffer::new(vec![0; 256]),
        }
    }

    pub fn update_at(&self, time: u64) {
        self.now.store(time, Ordering::Relaxed);
    }
}

pub struct DebugUiState {
    swap: DebugSwapState,
    cpu_mem: Vec<u8>,
    ppu_mem: Vec<u8>,
    pal_ram: Vec<u8>,
    sprite_ram: Vec<u8>,
    palette: Palette,
}

impl DebugUiState {
    pub fn new(swap: DebugSwapState, palette: Palette) -> Self {
        Self {
            swap,
            cpu_mem: vec![0; 0x10000],
            ppu_mem: vec![0; 0x4000],
            pal_ram: vec![0; 32],
            sprite_ram: vec![0; 256],
            palette,
        }
    }

    pub fn now(&self) -> u64 {
        self.swap.now.load(Ordering::Relaxed)
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

    pub fn ppu(&self) -> PpuView {
        PpuView(self)
    }

    pub fn swap(&mut self) {
        self.swap.cpu_mem.attempt_swap(&mut self.cpu_mem);
        self.swap.ppu_mem.attempt_swap(&mut self.ppu_mem);
        self.swap.pal_ram.attempt_swap(&mut self.pal_ram);
        self.swap.sprite_ram.attempt_swap(&mut self.sprite_ram);
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

    pub fn lookup(&self, idx: u8) -> (u8, u8, u8) {
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
    pub fn pal_entry(&self, idx: u8) -> (u8, u8, u8) {
        let idx = if idx & 3 == 0 { 0 } else { idx };
        let idx = self.pal_ram()[idx as usize];
        self.palette().lookup(idx)
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

        let chr_table = if self.cpu_mem()[0x2000] & 0x10 != 0 {
            ChrTable::High
        } else {
            ChrTable::Low
        };

        self.chr_tile(palette, chr_table, tile_idx)
    }
}

impl<'a> Deref for PpuView<'a> {
    type Target = DebugUiState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
