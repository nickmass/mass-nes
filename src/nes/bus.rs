use std::collections::HashMap;
use std::rc::Rc;
use nes::system::SystemState;

pub struct AddressBus {
    registered_reads: HashMap<u16, Box<Fn(&mut SystemState, u16) -> u8>>,
    registered_writes: HashMap<u16, Box<Fn(&mut SystemState, u16, u8)>>
}

impl AddressBus {
    pub fn new() -> AddressBus {
        AddressBus {
            registered_reads: HashMap::new(),
            registered_writes: HashMap::new(),
        }
    }

    pub fn register_read<T: AddressValidator, H: AddressReader>(&mut self, addr_val: T) {
        let iter = AddressIterator::new(addr_val);
        for addr in iter {
            let addr_closure = Box::new(|state: &mut SystemState, a: u16| H::read(state, a));
            self.registered_reads.insert(addr, addr_closure);
        }
    }

    pub fn register_write<T: AddressValidator, H: AddressWriter>(&mut self, addr_val: T) {
        let iter = AddressIterator::new(addr_val);
        for addr in iter {
            let addr_closure = Box::new(|state: &mut SystemState, a: u16, v: u8| H::write(state, a, v));
            self.registered_writes.insert(addr, addr_closure);
        }
    }
    
    pub fn read(&self, state: &mut SystemState, addr: u16) -> u8 {
        self.registered_reads.get(&addr).map_or(0, |handler| handler(state, addr))
    }
   
    pub fn read_word(&self, state: &mut SystemState, addr: u16) -> u16 {
        (self.read(state, addr) as u16) << 8 | self.read(state, addr + 1) as u16
    }

    pub fn write(&self, state: &mut SystemState, addr: u16, value: u8) {
        if let Some(handler) = self.registered_writes.get(&addr) {
            handler(state, addr, value);
        }
    }
}

pub struct SimpleAddress {
    address: u16
}

impl SimpleAddress {
    pub fn new(address: u16) -> SimpleAddress {
        SimpleAddress {
            address: address
        }
    }
}

impl AddressValidator for SimpleAddress {
    fn is_valid(&self, addr: u16) -> bool {
        self.address == addr
    }
}

pub trait AddressReader {
    fn read(&mut SystemState, u16) -> u8;
}

pub trait AddressWriter {
    fn write(&mut SystemState, u16, u8);
}

trait AddressValidator {
    fn is_valid(&self, u16) -> bool;
}

struct AddressIterator<T: AddressValidator> {
    addr_val: T,
    state: u32
}

impl<T: AddressValidator> AddressIterator<T> {
    fn new(val: T) -> AddressIterator<T> {
        AddressIterator { 
            addr_val: val,
            state: 0,
        }
    }
}

impl<T: AddressValidator> Iterator for AddressIterator<T> {
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
