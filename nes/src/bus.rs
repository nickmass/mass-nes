use crate::channel::Channel;
use crate::system::{System, SystemState};

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
    Nametables,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bus(usize);

#[derive(Clone, Copy, Debug)]
enum BusMapping {
    Mapped(u16, DeviceKind),
    Unmapped,
}

type Mapping = Vec<Vec<BusMapping>>;

pub struct DeviceMappings {
    read_mappings: Mapping,
    write_mappings: Mapping,
    next_bus: usize,
}

impl Default for DeviceMappings {
    fn default() -> DeviceMappings {
        DeviceMappings {
            read_mappings: Vec::new(),
            write_mappings: Vec::new(),
            next_bus: 0,
        }
    }
}

impl DeviceMappings {
    pub fn new() -> DeviceMappings {
        DeviceMappings {
            read_mappings: Vec::new(),
            write_mappings: Vec::new(),
            next_bus: 0,
        }
    }

    fn add_bus(&mut self) -> Bus {
        let bus = Bus(self.next_bus);
        self.next_bus += 1;

        self.read_mappings.push(vec![BusMapping::Unmapped; 0x10000]);
        self.write_mappings
            .push(vec![BusMapping::Unmapped; 0x10000]);

        bus
    }

    fn insert_read_mapping(&mut self, bus: &Bus, addr: u16, base_addr: u16, device: DeviceKind) {
        self.read_mappings.get_mut(bus.0).unwrap()[addr as usize] =
            BusMapping::Mapped(base_addr, device);
    }

    fn insert_write_mapping(&mut self, bus: &Bus, addr: u16, base_addr: u16, device: DeviceKind) {
        self.write_mappings.get_mut(bus.0).unwrap()[addr as usize] =
            BusMapping::Mapped(base_addr, device);
    }

    fn get_read_mapping(&self, bus: &Bus, addr: u16) -> Option<(u16, DeviceKind)> {
        match self.read_mappings[bus.0][addr as usize] {
            BusMapping::Mapped(x, y) => Some((x, y)),
            BusMapping::Unmapped => None,
        }
    }

    fn get_write_mapping(&self, bus: &Bus, addr: u16) -> Option<(u16, DeviceKind)> {
        match self.write_mappings[bus.0][addr as usize] {
            BusMapping::Mapped(x, y) => Some((x, y)),
            BusMapping::Unmapped => None,
        }
    }
}

pub struct AddressBus {
    kind: BusKind,
    bus: Bus,
    block_size: u16,
    mask: u16,
    open_bus: std::cell::Cell<u8>,
}

impl AddressBus {
    pub fn new(bus: BusKind, state: &mut SystemState, block_size: u32, mask: u16) -> AddressBus {
        AddressBus {
            kind: bus,
            bus: state.mappings.add_bus(),
            block_size: 2u16.pow(block_size),
            open_bus: std::cell::Cell::new(0),
            mask,
        }
    }

    pub fn register_read<T>(&self, state: &mut SystemState, device: DeviceKind, addr_val: T)
    where
        T: AddressValidator,
    {
        let mut addr: u32 = 0;
        while addr < 0x10000 {
            if let Some(base_addr) = addr_val.is_valid(addr as u16) {
                state
                    .mappings
                    .insert_read_mapping(&self.bus, addr as u16, base_addr, device);
            }
            addr += self.block_size as u32;
        }
    }

    pub fn register_write<T>(&self, state: &mut SystemState, device: DeviceKind, addr_val: T)
    where
        T: AddressValidator,
    {
        let mut addr: u32 = 0;
        while addr < 0x10000 {
            if let Some(base_addr) = addr_val.is_valid(addr as u16) {
                state
                    .mappings
                    .insert_write_mapping(&self.bus, addr as u16, base_addr, device);
            }
            addr += self.block_size as u32;
        }
    }

    pub fn peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = state.mappings.get_read_mapping(&self.bus, addr);
        match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.peek(state, addr),
            Some((addr, DeviceKind::Ppu)) => system.ppu.peek(system, state, addr),
            Some((addr, DeviceKind::Nametables)) => system.mapper.nt_peek(system, state, addr),
            Some((addr, DeviceKind::Mapper)) => system.mapper.peek(self.kind, system, state, addr),
            Some((addr, DeviceKind::Input)) => system.input.peek(addr, self.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => system.apu.peek(addr),
            None => self.open_bus.get(),
            _ => unimplemented!(),
        }
    }

    pub fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = state.mappings.get_read_mapping(&self.bus, addr);
        let value = match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.read(state, addr),
            Some((addr, DeviceKind::Ppu)) => system.ppu.read(system, state, addr),
            Some((addr, DeviceKind::Mapper)) => system.mapper.read(self.kind, system, state, addr),
            Some((addr, DeviceKind::Nametables)) => system.mapper.nt_read(system, state, addr),
            Some((addr, DeviceKind::Input)) => system.input.read(addr, self.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => system.apu.read(addr),
            None => self.open_bus.get(),
            _ => unimplemented!(),
        };

        self.open_bus.set(value);
        value
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let addr = (addr & !(self.block_size - 1)) & self.mask;
        let mapping = state.mappings.get_write_mapping(&self.bus, addr);
        match mapping {
            Some((addr, DeviceKind::CpuRam)) => system.cpu_mem.write(state, addr, value),
            Some((addr, DeviceKind::Ppu)) => system.ppu.write(system, state, addr, value),
            Some((addr, DeviceKind::Mapper)) => {
                system.mapper.write(self.kind, system, state, addr, value)
            }
            Some((addr, DeviceKind::Nametables)) => {
                system.mapper.nt_write(system, state, addr, value)
            }
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
    fn is_valid(&self, address: u16) -> Option<u16>;

    fn iter(&self) -> AddressIterator<Self>
    where
        Self: Sized,
    {
        AddressIterator::new(&self)
    }
}

pub struct AddressIterator<'a, T: 'a + AddressValidator> {
    addr_val: &'a T,
    state: i32,
}

impl<'a, T: AddressValidator> AddressIterator<'a, T> {
    fn new(val: &'a T) -> AddressIterator<'a, T> {
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
            if let Some(base) = self.addr_val.is_valid(x as u16) {
                return Some((x as u16, base));
            }
        }

        None
    }
}
