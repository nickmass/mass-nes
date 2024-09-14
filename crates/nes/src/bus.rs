use crate::channel::Channel;
use crate::Machine;

#[derive(Debug, Clone, Copy)]
pub enum BusKind {
    Cpu,
    Ppu,
}

#[derive(Clone, Copy, Debug)]
pub enum DeviceKind {
    CpuRam,
    Ppu,
    Mapper,
    Input,
    Expansion,
    Debug,
    Apu,
    PulseOne,
    PulseTwo,
    Noise,
    Triangle,
    Dmc,
}

struct DeviceMapping {
    mappings: Vec<(MappingFn, DeviceKind)>,
}

impl DeviceMapping {
    fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    fn insert<F: Into<MappingFn>>(&mut self, map_fn: F, device: DeviceKind) {
        self.mappings.push((map_fn.into(), device))
    }

    fn map(&self, addr: u16) -> Option<(u16, DeviceKind)> {
        for (map_fn, device) in self.mappings.iter() {
            if let Some(addr) = map_fn.map(addr) {
                return Some((addr, *device));
            }
        }

        None
    }
}

pub struct AddressBus {
    read_mapping: DeviceMapping,
    write_mapping: DeviceMapping,
    kind: BusKind,
    block_size: u16,
    mask: u16,
    open_bus: std::cell::Cell<u8>,
}

impl AddressBus {
    pub fn new(kind: BusKind, block_size: u32, mask: u16) -> AddressBus {
        AddressBus {
            read_mapping: DeviceMapping::new(),
            write_mapping: DeviceMapping::new(),
            kind,
            block_size: 2u16.pow(block_size),
            open_bus: std::cell::Cell::new(0),
            mask,
        }
    }

    pub fn register_read<T>(&mut self, device: DeviceKind, addr_val: T)
    where
        T: Into<MappingFn>,
    {
        self.read_mapping.insert(addr_val, device);
    }

    pub fn register_write<T>(&mut self, device: DeviceKind, addr_val: T)
    where
        T: Into<MappingFn>,
    {
        self.write_mapping.insert(addr_val, device);
    }

    pub fn peek(&self, system: &Machine, addr: u16) -> u8 {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = self.read_mapping.map(addr);
        match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.read(addr),
            Some((addr, DeviceKind::Ppu)) => system.ppu.peek(addr),
            Some((addr, DeviceKind::Mapper)) => system.mapper.peek(self.kind, addr),
            Some((addr, DeviceKind::Input)) => system.input.peek(addr, self.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => system.apu.peek(addr),
            None => self.open_bus.get(),
            _ => unimplemented!(),
        }
    }

    pub fn read(&self, system: &Machine, addr: u16) -> u8 {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = self.read_mapping.map(addr);
        let value = match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.read(addr),
            Some((addr, DeviceKind::Ppu)) => system.ppu.read(addr),
            Some((addr, DeviceKind::Mapper)) => system.mapper.read(self.kind, addr),
            Some((addr, DeviceKind::Input)) => system.input.read(addr, self.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => system.apu.read(addr),
            None => self.open_bus.get(),
            _ => unimplemented!(),
        };

        self.open_bus.set(value);
        value
    }

    pub fn write(&self, system: &Machine, addr: u16, value: u8) {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = self.write_mapping.map(addr);
        match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.write(addr, value),
            Some((addr, DeviceKind::Ppu)) => system.ppu.write(addr, value),
            Some((addr, DeviceKind::Mapper)) => system.mapper.write(self.kind, addr, value),
            Some((addr, DeviceKind::Input)) => system.input.write(addr, value),
            Some((addr, DeviceKind::Apu)) => system.apu.write(addr, value),
            Some((addr, DeviceKind::PulseOne)) => system.apu.pulse_one.write(addr, value),
            Some((addr, DeviceKind::PulseTwo)) => system.apu.pulse_two.write(addr, value),
            Some((addr, DeviceKind::Noise)) => system.apu.noise.write(addr, value),
            Some((addr, DeviceKind::Triangle)) => system.apu.triangle.write(addr, value),
            Some((addr, DeviceKind::Dmc)) => system.apu.dmc.write(addr, value),
            None => (),
            _ => unimplemented!(),
        }
    }

    pub fn peek_word(&self, system: &Machine, addr: u16) -> u16 {
        (self.peek(system, addr) as u16) | (self.peek(system, addr + 1) as u16) << 8
    }

    pub fn read_word(&self, system: &Machine, addr: u16) -> u16 {
        (self.read(system, addr) as u16) | (self.read(system, addr + 1) as u16) << 8
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Address(pub u16);

impl Address {
    fn map(&self, addr: u16) -> Option<u16> {
        if self.0 == addr {
            Some(addr)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AndAndMask(pub u16, pub u16);

impl AndAndMask {
    fn map(&self, addr: u16) -> Option<u16> {
        if addr & self.0 != 0 {
            Some(addr & self.1)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NotAndMask(pub u16);

impl NotAndMask {
    fn map(&self, addr: u16) -> Option<u16> {
        if addr & (!self.0) == 0 {
            Some(addr & self.0)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AndEqualsAndMask(pub u16, pub u16, pub u16);

impl AndEqualsAndMask {
    fn map(&self, addr: u16) -> Option<u16> {
        if addr & self.0 == self.1 {
            Some(addr & self.2)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RangeAndMask(pub u16, pub u16, pub u16);

impl RangeAndMask {
    fn map(&self, addr: u16) -> Option<u16> {
        if addr >= self.0 && addr < self.1 {
            Some(addr & self.2)
        } else {
            None
        }
    }
}

pub enum MappingFn {
    Address(Address),
    AndAndMask(AndAndMask),
    NotAndMask(NotAndMask),
    AndEqualsAndMask(AndEqualsAndMask),
    RangeAndMask(RangeAndMask),
}

impl MappingFn {
    fn map(&self, address: u16) -> Option<u16> {
        match self {
            MappingFn::Address(a) => a.map(address),
            MappingFn::AndAndMask(a) => a.map(address),
            MappingFn::NotAndMask(a) => a.map(address),
            MappingFn::AndEqualsAndMask(a) => a.map(address),
            MappingFn::RangeAndMask(a) => a.map(address),
        }
    }
}

impl From<Address> for MappingFn {
    fn from(value: Address) -> Self {
        MappingFn::Address(value)
    }
}

impl From<AndAndMask> for MappingFn {
    fn from(value: AndAndMask) -> Self {
        MappingFn::AndAndMask(value)
    }
}

impl From<NotAndMask> for MappingFn {
    fn from(value: NotAndMask) -> Self {
        MappingFn::NotAndMask(value)
    }
}

impl From<AndEqualsAndMask> for MappingFn {
    fn from(value: AndEqualsAndMask) -> Self {
        MappingFn::AndEqualsAndMask(value)
    }
}

impl From<RangeAndMask> for MappingFn {
    fn from(value: RangeAndMask) -> Self {
        MappingFn::RangeAndMask(value)
    }
}
