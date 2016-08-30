use nes::memory::Page;
use nes::bus::BusKind;
use nes::system::{System, SystemState};

pub struct NametablesState {
    mappings: [Nametable; 4],
}

impl Default for NametablesState {
    fn default() -> NametablesState {
        NametablesState {
            mappings: [Nametable::First,
                       Nametable::Second,
                       Nametable::First,
                       Nametable::Second]
        }
    }
}

pub struct Nametables {
    internal_first: Page,
    internal_second: Page,
}

#[derive(Copy, Clone)]
pub enum Nametable {
    First,
    Second,
    External(Page),
}

impl Nametables {
    pub fn new(state: &mut SystemState) -> Nametables {
        let first = state.mem.alloc_kb(1);
        let second = state.mem.alloc_kb(1);

        Nametables {
            internal_first: first,
            internal_second: second,
        }
    }
   
    pub fn set_vertical(&self, state: &mut SystemState) {
        state.ppu.nametables.mappings = [Nametable::First,
                                     Nametable::Second,
                                     Nametable::First,
                                     Nametable::Second];
    }
   
    pub fn set_horizontal(&self, state: &mut SystemState) {
        state.ppu.nametables.mappings = [Nametable::First,
                                     Nametable::First,
                                     Nametable::Second,
                                     Nametable::Second];
    }

    pub fn set_single(&self, state: &mut SystemState, nt: Nametable) {
        state.ppu.nametables.mappings = [nt, nt, nt, nt];
    }

    fn get_table(&self, state: &SystemState, addr: u16) -> (Page, u16) {
        let table_ind = (addr >> 10) & 3;
        let table_addr = addr & 0x3ff;
        let nametable = state.ppu.nametables.mappings[table_ind as usize];
        match nametable {
            Nametable::First => (self.internal_first, table_addr),
            Nametable::Second=> (self.internal_second, table_addr),
            Nametable::External(nt) => (nt, table_addr),
        }
    }

    pub fn peek(&self, bus: BusKind, state: &SystemState, addr: u16) -> u8 {
        let table = self.get_table(state, addr);
        state.mem.read(table.0, table.1)
    }

    pub fn read(&self, bus: BusKind, state: &mut SystemState, addr: u16) -> u8 {
        let table = self.get_table(state, addr);
        state.mem.read(table.0, table.1)
    }

    pub fn write(&self, bus: BusKind, state: &mut SystemState, addr: u16, val: u8) {
        let table = self.get_table(state, addr);
        state.mem.write(table.0, table.1, val);
    }
}
