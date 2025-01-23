#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::cartridge::INes;

use std::cell::Cell;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub struct Page {
    start: usize,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Pages {
    data: Vec<Cell<u8>>,
}

impl Pages {
    pub fn new() -> Pages {
        Pages { data: Vec::new() }
    }

    pub fn alloc_kb(&mut self, kb: usize) -> Page {
        let start = self.data.len();
        let end = start + (kb * 0x400);
        self.data.resize(end, Cell::new(0));
        Page { start }
    }

    pub fn read(&self, page: Page, addr: u16) -> u8 {
        let addr = page.start + addr as usize;
        self.data[addr].get()
    }

    pub fn write(&self, page: Page, addr: u16, val: u8) {
        let addr = page.start + addr as usize;
        self.data[addr].set(val);
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    mem: Pages,
    page: Page,
}

impl MemoryBlock {
    pub fn new(kb: usize) -> MemoryBlock {
        let mut mem = Pages::new();
        let page = mem.alloc_kb(kb);

        MemoryBlock { mem, page }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.mem.read(self.page, addr)
    }

    pub fn write(&self, addr: u16, val: u8) {
        self.mem.write(self.page, addr, val);
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum MemKind {
    Prg,
    Chr,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Banks {
    data: Vec<Page>,
    kind: MemKind,
}

impl Banks {
    pub fn load(cart: &INes, kind: MemKind) -> Banks {
        let data: &[u8] = match kind {
            MemKind::Prg => &*cart.prg_rom,
            MemKind::Chr => &*cart.chr_rom,
        };
        let mut v = Vec::new();
        let mut x = 0;
        while x + 0x400 <= data.len() {
            let start = x;
            x += 0x400;
            v.push(Page { start });
        }
        Banks { data: v, kind }
    }

    pub fn read(&self, cartridge: &INes, bank: usize, addr: u16) -> u8 {
        let bank = self.data[bank % self.data.len()];
        match self.kind {
            MemKind::Prg => cartridge.prg_rom[bank.start + addr as usize],
            MemKind::Chr => cartridge.chr_rom[bank.start + addr as usize],
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Mapped {
    Page(usize),
    Bank(usize),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BankKind {
    Ram,
    Rom,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MappedMemory {
    mem: Pages,
    banks: Banks,
    pages: Vec<Page>,
    base_addr: u16,
    mapping: Vec<Mapped>,
}

impl MappedMemory {
    pub fn new(
        cart: &INes,
        base_addr: u16,
        ram_kb: u32,
        size_kb: u32,
        kind: MemKind,
    ) -> MappedMemory {
        let mut mem = Pages::new();
        let mut pages = Vec::new();
        let mut mapping = Vec::new();

        for _ in 0..ram_kb {
            pages.push(mem.alloc_kb(1));
        }
        for _ in 0..size_kb {
            mapping.push(Mapped::Bank(0));
        }
        MappedMemory {
            mem,
            banks: Banks::load(cart, kind),
            pages,
            base_addr,
            mapping,
        }
    }

    pub fn map(&mut self, addr: u16, kb: u32, bank: usize, bank_kind: BankKind) {
        if addr & 0x3ff != 0 {
            panic!("Must map in 1kb chunks");
        }
        let offset = (addr - self.base_addr) / 0x400;
        let bank_start = bank * kb as usize;

        match bank_kind {
            BankKind::Rom => {
                for b in 0..kb as usize {
                    self.mapping[offset as usize + b] = Mapped::Bank(b + bank_start);
                }
            }
            BankKind::Ram => {
                for b in 0..kb as usize {
                    self.mapping[offset as usize + b] = Mapped::Page(b + bank_start);
                }
            }
        }
    }

    fn get_mapping(&self, addr: u16) -> Mapped {
        let offset = (addr - self.base_addr) / 0x400;
        self.mapping
            .get(offset as usize)
            .copied()
            .unwrap_or_else(|| {
                tracing::error!("bad mapping {:04x} : {:?}", addr, self.banks.kind);
                panic!("out of bounds")
            })
    }

    pub fn read(&self, cartridge: &INes, addr: u16) -> u8 {
        let mapping = self.get_mapping(addr);

        match mapping {
            Mapped::Bank(b) => self.banks.read(cartridge, b, addr & 0x3ff),
            Mapped::Page(p) => {
                let page = self.pages[p % self.pages.len()];
                self.mem.read(page, addr & 0x3ff)
            }
        }
    }

    pub fn write(&self, addr: u16, val: u8) {
        let mapping = self.get_mapping(addr);

        match mapping {
            Mapped::Bank(_) => {}
            Mapped::Page(p) => {
                let page = self.pages[p % self.pages.len()];
                self.mem.write(page, addr & 0x3ff, val);
            }
        }
    }
}
