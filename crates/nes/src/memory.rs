#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::SaveWram;

pub trait Memory {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn read(&self, address: usize) -> u8;
    fn write(&mut self, address: usize, value: u8);

    fn read_mapped(&self, bank: usize, bank_size: usize, addr: u16) -> u8 {
        assert_eq!(bank_size.count_ones(), 1, "bank_size must be power of 2");
        let low_addr = (addr as usize) & (bank_size - 1);
        let high_addr = bank << bank_size.trailing_zeros();
        let full_addr = high_addr | low_addr;

        self.read(full_addr)
    }

    fn write_mapped(&mut self, bank: usize, bank_size: usize, addr: u16, value: u8) {
        assert_eq!(bank_size.count_ones(), 1, "bank_size must be power of 2");
        let low_addr = (addr as usize) & (bank_size - 1);
        let high_addr = bank << bank_size.trailing_zeros();
        let full_addr = high_addr | low_addr;

        self.write(full_addr, value)
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    mem: Vec<u8>,
}

impl MemoryBlock {
    pub fn new(kb: usize) -> MemoryBlock {
        let mem = vec![0; kb << 10];

        MemoryBlock { mem }
    }

    pub fn save_wram(&self) -> Option<SaveWram> {
        Some(SaveWram::from_bytes(self.mem.clone()))
    }

    pub fn restore_wram(&mut self, wram: SaveWram) {
        let data = wram.to_bytes();

        for (a, b) in self.mem.iter_mut().zip(data) {
            *a = b;
        }
    }
}

impl Memory for MemoryBlock {
    fn len(&self) -> usize {
        self.mem.len()
    }

    fn read(&self, address: usize) -> u8 {
        let address = address % self.len();
        self.mem[address]
    }

    fn write(&mut self, address: usize, value: u8) {
        let address = address % self.len();
        self.mem[address] = value;
    }
}

pub struct RomBlock {
    rom: Vec<u8>,
}

impl RomBlock {
    pub fn new(rom: Vec<u8>) -> RomBlock {
        RomBlock { rom }
    }
}

impl Memory for RomBlock {
    fn len(&self) -> usize {
        self.rom.len()
    }

    fn read(&self, address: usize) -> u8 {
        let address = address % self.len();
        self.rom[address]
    }

    fn write(&mut self, _address: usize, _value: u8) {}
}
