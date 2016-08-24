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


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bus (u32);


type Mapping = HashMap<Bus, HashMap<u16, (u16, DeviceKind)>>;

pub struct DeviceMappings {
    read_mappings: Mapping,
    write_mappings: Mapping,
    next_bus: u32,
}

impl Default for DeviceMappings {
    fn default() -> DeviceMappings {
        DeviceMappings {
            read_mappings: HashMap::new(),
            write_mappings: HashMap::new(),
            next_bus: 0,
        }
    }
}

impl DeviceMappings {
    pub fn new() -> DeviceMappings {
        DeviceMappings {
            read_mappings: HashMap::new(),
            write_mappings: HashMap::new(),
            next_bus: 0,
        }
    }

    fn add_bus(&mut self) -> Bus {
        let bus = Bus(self.next_bus);
        self.next_bus += 1;

        self.read_mappings.insert(bus, HashMap::new());
        self.write_mappings.insert(bus, HashMap::new());

        bus
    }

    fn insert_read_mapping(&mut self,
                           bus: &Bus, addr: u16, base_addr: u16, device: DeviceKind) {
        self.read_mappings.get_mut(bus).unwrap().insert(addr, (base_addr, device));
    }
    
    fn insert_write_mapping(&mut self,
                           bus: &Bus, addr: u16, base_addr: u16, device: DeviceKind) {
        self.write_mappings.get_mut(bus).unwrap().insert(addr, (base_addr, device));
    }
    
    fn get_read_mapping(&self, bus: &Bus, addr: &u16) -> Option<(u16, DeviceKind)> {
        self.read_mappings[bus].get(addr).map(|x| (x.0, x.1))
    }

    fn get_write_mapping(&self, bus: &Bus, addr: &u16) -> Option<(u16, DeviceKind)> {
        self.write_mappings[bus].get(addr).map(|x| (x.0, x.1))
    }
    
}


pub struct AddressBus {
    kind: BusKind,
    bus: Bus,
    block_size: u16
}

impl AddressBus {
    pub fn new(bus: BusKind, state: &mut SystemState, block_size: u32) -> AddressBus {
        AddressBus {
            kind: bus,
            bus: state.mappings.add_bus(),
            block_size: 2u16.pow(block_size)
        }
    }

    pub fn register_read<T>(&self, state: &mut SystemState, device: DeviceKind, addr_val: T)
            where T: AddressValidator {
        let mut addr: u32 = 0;
        while addr < 0x10000 {
            match addr_val.is_valid(addr as u16) {
                Some(base_addr) => {
                    state.mappings
                        .insert_read_mapping(&self.bus, addr as u16, base_addr, device);
                },
                None => {}
            }
            addr += self.block_size as u32;
        }
    }

    pub fn register_write<T>(&self, state: &mut SystemState, device: DeviceKind, addr_val: T)
            where T: AddressValidator {
        let mut addr: u32 = 0;
        while addr < 0x10000 {
            match addr_val.is_valid(addr as u16) {
                Some(base_addr) => {
                    state.mappings
                        .insert_write_mapping(&self.bus, addr as u16, base_addr, device);
                },
                None => {}
            }
            addr += self.block_size as u32;
        }
    }
    
    pub fn peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        let addr = addr & !(self.block_size - 1);
        let mapping = state.mappings.get_read_mapping(&self.bus, &addr);
        match mapping {
            Some(h) => {
                match h.1 {
                    DeviceKind::CpuRam => system.cpu.mem.peek(self.kind, state, h.0),
                    DeviceKind::Ppu => system.ppu.peek(self.kind, system, state, h.0),
                    DeviceKind::PpuRam => system.ppu.mem.peek(self.kind, state, h.0),
                    DeviceKind::Mapper => system.cartridge.peek(self.kind, state, h.0),
                    _ => unimplemented!(),
                }
            },
            None => {
                0xff
            }
        }
    }
    
    pub fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        let addr = addr & !(self.block_size - 1);
        let mapping = state.mappings.get_read_mapping(&self.bus, &addr);
        match mapping {
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
                0xff
            }
        }
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let addr = addr & !(self.block_size - 1); 
        let mapping = state.mappings.get_write_mapping(&self.bus, &addr);
        match mapping {
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

    pub fn peek_word(&self, system: &System, state: &SystemState, addr: u16) -> u16 {
        (self.peek(system, state, addr) as u16) | (self.peek(system, state, addr + 1) as u16) << 8
    }

    pub fn read_word(&self, system: &System, state: &mut SystemState, addr: u16) -> u16 {
        (self.read(system, state, addr) as u16) | (self.read(system, state, addr + 1) as u16) << 8
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
