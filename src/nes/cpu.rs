use nes::bus::{BusKind, DeviceKind, AddressBus, AddressValidator, Address};
use nes::system::{System, SystemState};
use nes::ops::*;
use nes::memory::MemoryBlock;

use std::collections::{VecDeque, HashMap};

#[derive(Default)]
pub struct CpuState {
    current_tick: u64,
    stage: Stage,
    decode_stack: VecDeque<u8>,
    op_addr: u16,
    op: Op,
    reg_a: u32,
    reg_x: u32,
    reg_y: u32,
    reg_pc: u32,
    reg_sp: u32,
}

#[derive(Copy, Clone)]
enum Stage {
    Fetch,
    Address(u32),
    Execute(u32),
}

impl Stage {
    fn increment(self) -> Stage {
        match self {
            Stage::Fetch => unreachable!(),
            Stage::Address(n) => Stage::Address(n + 1),
            Stage::Execute(n) => Stage::Execute(n + 1),
        }
    }
}

impl Default for Stage {
    fn default() -> Stage {
        Stage::Fetch
    }
}

pub struct Cpu {
    bus: AddressBus,
    pub mem: MemoryBlock,
    ops: HashMap<u8, Op>,
}

impl Cpu {
    pub fn new(state: &mut SystemState) -> Cpu {
        Cpu {
            bus: AddressBus::new(BusKind::Cpu),
            mem: MemoryBlock::new(2, &mut state.mem),
            ops: Op::load(),
        }
    }

    pub fn register_read<T>(&mut self, device: DeviceKind, addr: T)
        where T: AddressValidator {
        self.bus.register_read(device, addr);
    }

    pub fn register_write<T>(&mut self, device: DeviceKind, addr: T)
        where T: AddressValidator {
        self.bus.register_write(device, addr);
    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.cpu.current_tick += 1;
        match state.cpu.stage {
            Stage::Fetch => {
                self.decode(system, state);
            },
            Stage::Address(_) => {
                self.addressing(system, state);
            },
            Stage::Execute(_) => {
                self.operation(system, state);
            },
        }
    }
    
    fn read_pc(&self, system: &System, state: &mut SystemState) -> u8 {
        let pc = (state.cpu.reg_pc + 1) as u16;
        state.cpu.reg_pc = pc as u32;
        self.bus.read(system, state, pc)
    }

    fn decode(&self, system: &System, state: &mut SystemState) {
        state.cpu.op = self.ops[&self.read_pc(system, state)];
    }

    fn addressing(&self, system: &System, state: &mut SystemState) {
        let current = (state.cpu.op.addressing, state.cpu.stage);
        match current {
            (Addressing::None, Stage::Address(0)) => {
                let r = state.cpu.reg_pc + 1;
                let _ = self.bus.read(system, state, r as u16);
            },
            (Addressing::Accumulator, Stage::Address(0)) => {
                let r = state.cpu.reg_pc + 1;
                let _ = self.bus.read(system, state, r as u16);
                state.cpu.op_addr = state.cpu.reg_a as u16;      
            },
            (Addressing::Immediate, Stage::Address(0)) => {
                state.cpu.reg_pc += 1;
                state.cpu.op_addr = state.cpu.reg_pc as u16;
                state.cpu.stage = Stage::Execute(0);
                self.operation(system, state);
                return;
            },
            (Addressing::ZeroPage, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::ZeroPageX, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::ZeroPageX, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = (state.cpu.op_addr as u32 + state.cpu.reg_x) as u16;
                state.cpu.op_addr = a;
            },
            (Addressing::ZeroPageY, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::ZeroPageY, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = (state.cpu.op_addr as u32 + state.cpu.reg_y) as u16;
                state.cpu.op_addr = a;
            },
            (Addressing::Absolute, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;

            },
            (Addressing::Absolute, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::AbsoluteX, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;

            },
            (Addressing::AbsoluteX, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::AbsoluteX, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                if (a & 0xFF00) !=
                   ((a + state.cpu.reg_x as u16) & 0xFF) {
                    let dummy_a = (a & 0xFF00) | ((a + state.cpu.reg_x as u16) & 0xFF);
                    let _ = self.bus.read(system, state, dummy_a);
                    state.cpu.op_addr += state.cpu.reg_x as u16;
                } else {
                    state.cpu.op_addr += state.cpu.reg_x as u16;
                    state.cpu.stage = Stage::Execute(0);
                    self.operation(system, state);
                    return;
                }
            },
            (Addressing::AbsoluteY, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;

            },
            (Addressing::AbsoluteY, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::AbsoluteY, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                if (a & 0xFF00) !=
                   ((a + state.cpu.reg_y as u16) & 0xFF) {
                    let dummy_a = (a & 0xFF00) | ((a + state.cpu.reg_y as u16) & 0xFF);
                    let _ = self.bus.read(system, state, dummy_a);
                    state.cpu.op_addr += state.cpu.reg_y as u16;
                } else {
                    state.cpu.op_addr += state.cpu.reg_y as u16;
                    state.cpu.stage = Stage::Execute(0);
                    self.operation(system, state);
                    return;
                }
            },
            (Addressing::AbsoluteXDummyAlways, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;

            },
            (Addressing::AbsoluteXDummyAlways, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::AbsoluteXDummyAlways, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                let dummy_a = (a & 0xFF00) | ((a + state.cpu.reg_x as u16) & 0xFF);
                let _ = self.bus.read(system, state, dummy_a);
                state.cpu.op_addr += state.cpu.reg_x as u16;
            },
            (Addressing::AbsoluteYDummyAlways, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;

            },
            (Addressing::AbsoluteYDummyAlways, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::AbsoluteYDummyAlways, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                let dummy_a = (a & 0xFF00) | ((a + state.cpu.reg_y as u16) & 0xFF);
                let _ = self.bus.read(system, state, dummy_a);
                state.cpu.op_addr += state.cpu.reg_y as u16;
            },
            (Addressing::IndirectAbsolute, Stage::Address(0)) => { 
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::IndirectAbsolute, Stage::Address(1)) => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
            },
            (Addressing::IndirectAbsolute, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
            },
            (Addressing::IndirectAbsolute, Stage::Address(3)) => {
                let a = (state.cpu.op_addr & 0xff00) | ((state.cpu.op_addr + 1) & 0xff);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a | state.cpu.decode_stack.pop_back().unwrap() as u16;
            },
            (Addressing::Relative, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::IndirectX, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::IndirectX, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = a + state.cpu.reg_x as u16;
                let a = a & 0xff;
                state.cpu.op_addr = a;
            },
            (Addressing::IndirectX, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
            },
            (Addressing::IndirectX, Stage::Address(3)) => {
                let a = (state.cpu.op_addr & 0xff00) | ((state.cpu.op_addr + 1) & 0xff);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a | state.cpu.decode_stack.pop_back().unwrap() as u16;
            },
            (Addressing::IndirectY, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::IndirectY, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
            },
            (Addressing::IndirectY, Stage::Address(2)) => {
                let a = (state.cpu.op_addr & 0xff00) | ((state.cpu.op_addr + 1) & 0xff);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a | state.cpu.decode_stack.pop_back().unwrap() as u16;
            },
            (Addressing::AbsoluteY, Stage::Address(3)) => {
                let a = state.cpu.op_addr;
                if (a & 0xff00) !=
                   ((a + state.cpu.reg_y as u16) & 0xff) {
                    let dummy_a = (a & 0xff00) | ((a + state.cpu.reg_y as u16) & 0xff);
                    let _ = self.bus.read(system, state, dummy_a);
                    state.cpu.op_addr += state.cpu.reg_y as u16;
                } else {
                    state.cpu.op_addr += state.cpu.reg_y as u16;
                    state.cpu.stage = Stage::Execute(0);
                    self.operation(system, state);
                    return;
                }
            },
            
            _ => {
                state.cpu.stage = Stage::Execute(0);
                self.operation(system, state);
                return;
            },
        }
        state.cpu.stage = current.1.increment();
    }

    fn operation(&self, system: &System, state: &mut SystemState) {
    }

}

