use std::collections::HashMap;
use std::rc::Rc;
use nes::system::SystemState;
use nes::system::System;

#[derive(Clone, Copy)]
pub enum BusKind {
    Cpu,
    Ppu,
}

#[derive(Clone, Copy, Debug)]
pub enum DeviceKind {
    CpuRam,
    PpuRam,
    Ppu,
    Mapper,
    Input(i32),
    Expansion,
    Debug,
}

pub struct AddressBus {
    kind: BusKind,
    registered_reads: HashMap<u16, (u16, DeviceKind)>,
    registered_writes: HashMap<u16, (u16, DeviceKind)>,
}

impl AddressBus {
    pub fn new(bus: BusKind) -> AddressBus {
        AddressBus {
            kind: bus,
            registered_reads: HashMap::new(),
            registered_writes: HashMap::new(),
        }
    }

    pub fn register_read<T>(&mut self, device: DeviceKind, addr_val: T)
            where T: AddressValidator {
        for (addr, base_addr) in addr_val.iter() {
            self.registered_reads.insert(addr, (base_addr, device));
        }
    }

    pub fn register_write<T>(&mut self, device: DeviceKind, addr_val: T)
            where T: AddressValidator {
        for (addr, base_addr) in addr_val.iter() {
            self.registered_writes.insert(addr, (base_addr, device));
        }
    }
    
    pub fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match self.registered_reads.get(&addr) {
            Some(h) => {
                match h.1 {
                    DeviceKind::CpuRam => system.cpu.mem.read(self.kind, state, h.0),
                    DeviceKind::Ppu => system.ppu.read(self.kind, system, state, h.0),
                    DeviceKind::PpuRam => system.ppu.mem.read(self.kind, state, h.0),
                    DeviceKind::Mapper => system.cartridge.read(self.kind, state, h.0),
                    _ => unimplemented!(),
                }
            },
            None => {
                0xFF
            }
        }

    }
   
    pub fn read_word(&self, system: &System, state: &mut SystemState, addr: u16) -> u16 {
        (self.read(system, state, addr) as u16) << 8 | self.read(system, state, addr + 1) as u16
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match self.registered_writes.get(&addr) {
            Some(h) => {
                match h.1  {
                    DeviceKind::CpuRam => system.cpu.mem.write(self.kind,  state, h.0, value),
                    DeviceKind::Ppu => system.ppu.write(self.kind, system, state, h.0, value),
                    DeviceKind::PpuRam => system.ppu.mem.write(self.kind, state, h.0, value),
                    DeviceKind::Mapper => system.cartridge.write(self.kind, state, h.0, value),
                    _ => unimplemented!(),
                }
            },
            None => {}
        }
    }
}

pub struct Address(pub u16);

impl AddressValidator for Address {
    fn is_valid(&self, addr: u16) -> Option<u16> {
        if self.0 == addr {
            Some(addr)
        } else {
            None
        }
    }
}

pub struct AndAndMask(pub u16, pub u16);

impl AddressValidator for AndAndMask {
    fn is_valid(&self, addr: u16) -> Option<u16> {
        if addr & self.0 != 0 {
            Some(addr & self.1)
        } else {
            None
        }
    }
}

pub struct NotAndMask(pub u16);

impl AddressValidator for NotAndMask {
    fn is_valid(&self, addr: u16) -> Option<u16> {
        if addr & (!self.0) == 0 {
            Some(addr & self.0)
        } else {
            None
        }
    }
}

pub struct AndEqualsAndMask(pub u16, pub u16, pub u16);

impl AddressValidator for AndEqualsAndMask {
    fn is_valid(&self, addr: u16) -> Option<u16> {
        if addr & self.0 == self.1 {
            Some(addr & self.2)
        } else {
            None
        }
    }
}

pub struct RangeAndMask(pub u16, pub u16, pub u16);

impl AddressValidator for RangeAndMask {
    fn is_valid(&self, addr: u16) -> Option<u16> {
        if addr >= self.0 && addr < self.1 {
            Some(addr & self.2)
        } else {
            None
        }
    }
}

pub trait AddressValidator {
    fn is_valid(&self, u16) -> Option<u16>;

    fn iter(&self) -> AddressIterator<Self> where Self: Sized {
        AddressIterator::new(&self)
    }
}

pub struct AddressIterator<'a, T: 'a + AddressValidator> {
    addr_val: &'a T,
    state: i32
}

impl<'a, T: AddressValidator> AddressIterator<'a, T> {
    fn new(val: &'a T) -> AddressIterator<'a, T>  {
        AddressIterator { 
            addr_val: val,
            state: -1,
        }
    }
}

impl<'a, T: AddressValidator> Iterator for AddressIterator<'a, T> {
    type Item = (u16, u16);

    fn next(&mut self) -> Option<(u16, u16)> {
        let start = self.state + 1;
        for x in start..0x10000 {
            self.state = x;
            match self.addr_val.is_valid(x as u16) {
                Some(base) => { return Some((x as u16, base)); },
                None => {}
            }
        }

        None
    }
}
