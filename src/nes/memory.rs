use nes::system::SystemState;
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
        let end = start + (kb * 0x200);
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
            page: pages.alloc_kb(2),
        }
    }

    pub fn read(&self, bus: BusKind, state: &mut SystemState, addr: u16) -> u8 {
        state.mem.read(self.page, addr)
    }

    pub fn write(&self, bus: BusKind, state: &mut SystemState, addr: u16, val: u8) {
        state.mem.write(self.page, addr, val);
    }
}
