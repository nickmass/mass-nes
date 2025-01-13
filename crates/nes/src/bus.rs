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
    Apu,
    PulseOne,
    PulseTwo,
    Noise,
    Triangle,
    Dmc,
    Debug,
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
        self.iter(addr).next()
    }

    fn iter(&self, addr: u16) -> impl Iterator<Item = (u16, DeviceKind)> + '_ {
        self.mappings
            .iter()
            .filter_map(move |(map_fn, device)| map_fn.map(addr).zip(Some(*device)))
    }
}

pub struct AddressBus {
    read_mapping: DeviceMapping,
    write_mapping: DeviceMapping,
    block_size: u16,
    mask: u16,
    pub(crate) open_bus: std::cell::Cell<u8>,
}

impl AddressBus {
    pub(crate) fn new(block_size: u32, mask: u16) -> AddressBus {
        AddressBus {
            read_mapping: DeviceMapping::new(),
            write_mapping: DeviceMapping::new(),
            block_size: 2u16.pow(block_size),
            open_bus: std::cell::Cell::new(0),
            mask,
        }
    }

    pub(crate) fn register_read<T>(&mut self, device: DeviceKind, addr_val: T)
    where
        T: Into<MappingFn>,
    {
        self.read_mapping.insert(addr_val, device);
    }

    pub(crate) fn register_write<T>(&mut self, device: DeviceKind, addr_val: T)
    where
        T: Into<MappingFn>,
    {
        self.write_mapping.insert(addr_val, device);
    }

    pub(crate) fn read_addr(&self, addr: u16) -> Option<(u16, DeviceKind)> {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        self.read_mapping.map(addr)
    }

    pub(crate) fn write_addrs(&self, addr: u16) -> impl Iterator<Item = (u16, DeviceKind)> + '_ {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        self.write_mapping.iter(addr)
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
