use std::collections::HashMap;
use std::rc::Rc;

pub struct AddressBus {
    registered_reads: HashMap<u16, Box<Fn(u16) -> u8>>,
    registered_writes: HashMap<u16, Box<Fn(u16, u8)>>
}

impl AddressBus {
    pub fn new() -> AddressBus {
        AddressBus {
            registered_reads: HashMap::new(),
            registered_writes: HashMap::new(),
        }
    }

    pub fn register_read<T: AddressValidator, H: AddressReader>(&self, addr_val: T, handler: Rc<H>) {
        let iter = AddressIterator::new(addr_val);
        let mut reads = self.registered_reads;
        for addr in iter {
            let addr_handler = handler.clone();
            let addr_closure = Box::new(move |a: u16| addr_handler.read(a));
            reads.insert(addr, addr_closure);
        }
    }

    pub fn register_write<T: AddressValidator, H: AddressWriter>(&self, addr_val: T, handler: Rc<H>) {
        let iter = AddressIterator::new(addr_val);
        let mut writes = self.registered_writes;
        for addr in iter {
            let addr_handler = handler.clone();
            let addr_closure = Box::new(move |a: u16, v: u8| addr_handler.write(a, v));
            writes.insert(addr, addr_closure);
        }
    }
}

impl AddressReader for AddressBus {
    fn read(&self, addr: u16) -> u8 {
        self.registered_reads.get(&addr).map_or(0, |handler| handler(addr))
    }
}

impl AddressWriter for AddressBus {
    fn write(&self, addr: u16, value: u8) {
        let handler = self.registered_writes.get(&addr);
        if handler.is_some() {
            handler.unwrap()(addr, value);
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
    fn read(&self, u16) -> u8;

    fn read_word(&self, addr: u16) -> u16 {
        (self.read(addr) as u16) << 8 | self.read(addr + 1) as u16
    }
}

pub trait AddressWriter {
    fn write(&self, u16, u8);
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
                return Some(x as u16);
            }
        }

        None
    }
}
