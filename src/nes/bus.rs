use std::collections::HashMap;
use std::rc::Rc;
use nes::system::SystemState;
use nes::system::System;

#[derive(Clone, Copy)]
pub enum BusKind {
    Cpu,
    Ppu,
}

#[derive(Clone, Copy)]
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
    registered_reads: HashMap<u16, DeviceKind>,
    registered_writes: HashMap<u16, DeviceKind>,
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
        for addr in addr_val.iter() {
            self.registered_reads.insert(addr, device);
        }
    }

    pub fn register_write<T>(&mut self, device: DeviceKind, addr_val: T)
            where T: AddressValidator {
        for addr in addr_val.iter() {
            self.registered_writes.insert(addr, device);
        }
    }
    
    pub fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match self.registered_reads.get(&addr) {
            Some(h) => {
                match *h {
                    DeviceKind::CpuRam => system.cpu.mem.read(self.kind, state, addr),
                    DeviceKind::Ppu => system.ppu.read(self.kind, state, addr),
                    //PpuRam => system.ppu.ram.read(self.kind, state, addr),
                    //Mapper => system.mapper.read(self.kind, state, addr),
                    _ => unimplemented!(),
                }
            },
            None => 0xff
        }

    }
   
    pub fn read_word(&self, system: &System, state: &mut SystemState, addr: u16) -> u16 {
        (self.read(system, state, addr) as u16) << 8 | self.read(system, state, addr + 1) as u16
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        match self.registered_writes.get(&addr) {
            Some(h) => {
                match *h  {
                    DeviceKind::CpuRam => system.cpu.mem.write(self.kind, state, addr, value),
                    DeviceKind::Ppu => system.ppu.write(self.kind, state, addr, value),
                    //PpuRam => system.ppu.ram.write(self.kind, state, addr, value),
                    //Mapper => system.mapper.write(self.kind, state, addr, value),
                    _ => unimplemented!(),
                }
            },
            None => {}
        }
    }
}

pub struct Address(pub u16);

impl AddressValidator for Address {
    fn is_valid(&self, addr: u16) -> bool {
        self.0 == addr
    }
}

pub trait AddressValidator {
    fn is_valid(&self, u16) -> bool;

    fn iter(&self) -> AddressIterator<Self> where Self: Sized {
        AddressIterator::new(&self)
    }
}

pub struct AddressIterator<'a, T: 'a + AddressValidator> {
    addr_val: &'a T,
    state: u32
}

impl<'a, T: AddressValidator> AddressIterator<'a, T> {
    fn new(val: &'a T) -> AddressIterator<'a, T>  {
        AddressIterator { 
            addr_val: val,
            state: 0,
        }
    }
}

impl<'a, T: AddressValidator> Iterator for AddressIterator<'a, T> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        let start = self.state + 1;
        for x in start..0x10000 {
            if self.addr_val.is_valid(x as u16) {
                self.state = x;
                return Some(x as u16);
            }
        }

        None
    }
}
