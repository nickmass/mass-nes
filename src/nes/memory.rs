use nes::system::{System, SystemState};
use nes::cartridge::Cartridge;
use nes::bus::BusKind;

#[derive(Copy, Clone)]
pub struct Page {
    start: usize,
    end: usize,
}

#[derive(Default)]
pub struct Pages {
    current_size: usize,
    data: Vec<u8>,
}

impl Pages {
    pub fn new() -> Pages {
        Pages {
            current_size: 0,
            data: Vec::new(),
        }
    }

    pub fn alloc_kb(&mut self, kb: usize) -> Page {
        let start = self.data.len();
        let end = start + (kb * 0x400);
        self.data.resize(end, 0);
        Page { start: start, end: end }
    }

    pub fn read(&self, page: Page, addr: u16) -> u8 {
        self.data[page.start + addr as usize]
    }

    pub fn write(&mut self, page: Page, addr: u16, val: u8) {
        self.data[page.start + addr as usize] = val;
    }
}

pub struct MemoryBlock {
    kb: usize,
    page: Page
}

impl MemoryBlock {
    pub fn new(kb: usize, pages: &mut Pages) -> MemoryBlock {
        MemoryBlock {
            kb: kb,
            page: pages.alloc_kb(kb),
        }
    }

    pub fn peek(&self, state: &SystemState, addr: u16) -> u8 {
        state.mem.read(self.page, addr)
    }

    pub fn read(&self, state: &SystemState, addr: u16) -> u8 {
        state.mem.read(self.page, addr)
    }

    pub fn write(&self, state: &mut SystemState, addr: u16, val: u8) {
        state.mem.write(self.page, addr, val);
    }
}

pub enum MemKind {
    Prg,
    Chr,
}

pub struct Banks {
    data: Vec<Page>,
    kind: MemKind,
}

impl Banks {
    pub fn load(cart: &Cartridge, kind: MemKind) -> Banks {
        let data: &[u8] = match kind {
            MemKind::Prg => &*cart.prg_rom,
            MemKind::Chr => &*cart.chr_rom,
        };
        let mut v = Vec::new();
        let mut x = 0;
        while x + 0x400 <= data.len() {
            let start = x;
            x += 0x400;
            let end = x;
            v.push(Page { start: start, end: end });
        }
        Banks {
            data: v,
            kind: kind,
        }
    }

    pub fn read(&self, system: &System, bank: usize, addr: u16) -> u8 {
        let bank = self.data[bank];
        match self.kind {
            MemKind::Prg =>
                system.cartridge.prg_rom[bank.start + addr as usize],
            MemKind::Chr => 
                system.cartridge.chr_rom[bank.start + addr as usize],
        }
    }
}

#[derive(Copy, Clone)]
enum Mapped {
    Page(usize),
    Bank(usize),
}

#[derive(Copy, Clone)]
pub enum BankKind {
    Ram,
    Rom,
}

pub struct MappedMemory {
    banks: Banks,
    pages: Vec<Page>,
    base_addr: u16,
    mapping: Vec<Mapped>,
}

impl MappedMemory {
    pub fn new(state: &mut SystemState, cart: &Cartridge, base_addr: u16,
               ram_kb: u32, size_kb: u32, kind: MemKind) -> MappedMemory {
        let mut pages = Vec::new();
        let mut mapping = Vec::new();

        for x in 0..ram_kb {
            pages.push(state.mem.alloc_kb(1));
        }
        for x in 0..size_kb {
            mapping.push(Mapped::Bank(0));
        }
        MappedMemory {
            banks: Banks::load(cart, kind),
            pages: pages,
            base_addr: base_addr,
            mapping: mapping,
        }
    }

    pub fn map(&mut self, addr: u16, kb: u32, bank: usize, bank_kind: BankKind) {
        if addr & 0x3ff != 0 { panic!("Must map in 1kb chunks"); }
        let offset = (addr - self.base_addr) / 0x400;
        let bank_start = (bank * kb as usize) ;//% self.mapping.len();

        match bank_kind {
            BankKind::Rom => {
                for b in 0..kb as usize {
                    self.mapping[offset as usize + b] = Mapped::Bank(b + bank_start);
                }
            },
            BankKind::Ram => {
                for b in 0..kb as usize{
                    self.mapping[offset as usize + b] = Mapped::Page(b + bank_start);
                }
            }
        }
    }

    fn get_mapping(&self, addr: u16) -> Mapped {
        let offset = (addr - self.base_addr) / 0x400;
        self.mapping[offset as usize]
    }

    pub fn read(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        let mapping = self.get_mapping(addr);

        match mapping {
            Mapped::Bank(b) => {
                self.banks.read(system, b, addr & 0x3ff)
            },
            Mapped::Page(p) => {
                let page = self.pages[p];
                state.mem.read(page, addr & 0x3ff)
            }
        }
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16, val: u8) {
        let mapping = self.get_mapping(addr);

        match mapping {
            Mapped::Bank(b) => {},
            Mapped::Page(p) => {
                let page = self.pages[p];
                state.mem.write(page, addr & 0x3ff, val);
            }
        }
    }
}
