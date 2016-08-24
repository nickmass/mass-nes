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
    flag_c: u32,
    flag_z: u32,
    flag_i: u32,
    flag_d: u32,
    flag_v: u32,
    flag_s: u32,
    pending_oam_dma: Option<u8>,
    oam_dma_buffer: u8,

    pending_nmi: bool,
}

impl CpuState {
    fn reg_p(&self) -> u8 {
        let mut val = 0;
        if self.flag_c != 0 { val |= 0x01; }
        if self.flag_z == 0 { val |= 0x02; }
        if self.flag_i != 0 { val |= 0x04; }
        if self.flag_d != 0 { val |= 0x08; }
        if self.flag_v != 0 { val |= 0x40; }
        if self.flag_s & 0x80 != 0 { val |= 0x80; }

        val
    }

    fn set_reg_p(&mut self, val: u32) {
        self.flag_c = val & 0x01;
        self.flag_z = (val & 0x02) ^ 0x02;
        self.flag_i = val & 0x04;
        self.flag_d = val & 0x08;
        self.flag_v = val & 0x40;
        self.flag_s = val & 0x80;
    }

    pub fn oam_dma_req(&mut self, addr: u8){
        self.pending_oam_dma = Some(addr);
    }

    pub fn nmi_req(&mut self) {
        self.pending_nmi = true;
    }
}

#[derive(Copy, Clone)]
enum Stage {
    Fetch,
    Address(u32),
    Execute(u32),
    OamDma(u32),
    Nmi(u32),
}

impl Stage {
    fn increment(self) -> Stage {
        match self {
            Stage::Fetch => unreachable!(),
            Stage::Address(n) => Stage::Address(n + 1),
            Stage::Execute(n) => Stage::Execute(n + 1),
            Stage::OamDma(n) => Stage::OamDma(n + 1),
            Stage::Nmi(n) => {
                if n == 6 {
                    Stage::Fetch
                } else {
                    Stage::Nmi(n + 1)
                }
            }
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
        state.cpu.reg_pc = 0xc000;
        state.cpu.reg_sp = 0xfd; 
        Cpu {
            bus: AddressBus::new(BusKind::Cpu, state, 0),
            mem: MemoryBlock::new(2, &mut state.mem),
            ops: Op::load(),
        }
    }

    pub fn register_read<T>(&mut self, state: &mut SystemState, device: DeviceKind, addr: T)
        where T: AddressValidator {
        self.bus.register_read(state, device, addr);
    }

    pub fn register_write<T>(&mut self, state: &mut SystemState, device: DeviceKind, addr: T)
        where T: AddressValidator {
        self.bus.register_write(state, device, addr);
    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.cpu.current_tick += 1;
        if state.cpu.pending_nmi {
            state.cpu.decode_stack.clear();
            state.cpu.stage = Stage::Nmi(0);
            state.cpu.pending_nmi = false;
        }
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
            Stage::OamDma(_) => {
                self.oam_dma(system, state);
            },
            Stage::Nmi(_) => {
                self.nmi(system, state);
            },
        }
    }
    
    fn read_pc(&self, system: &System, state: &mut SystemState) -> u8 {
        let pc = state.cpu.reg_pc as u16;
        let value = self.bus.read(system, state, pc);
        state.cpu.reg_pc = pc.wrapping_add(1) as u32;
        value
    }

    fn push_stack(&self, system: &System, state: &mut SystemState, value :u8) {
        let addr = state.cpu.reg_sp as u16 | 0x100;
        self.bus.write(system, state, addr, value);
        state.cpu.reg_sp = state.cpu.reg_sp.wrapping_sub(1);
        state.cpu.reg_sp &= 0xff;
    }

    fn pop_stack(&self, system: &System, state: &mut SystemState) -> u8 {
        state.cpu.reg_sp = state.cpu.reg_sp.wrapping_add(1);
        state.cpu.reg_sp &= 0xff;
        let addr = state.cpu.reg_sp as u16 | 0x100;
        self.bus.read(system, state, addr)
    }

    fn oam_dma(&self, system: &System, state: &mut SystemState) {
        let high_addr = state.cpu.pending_oam_dma.unwrap();
        let high_addr = (high_addr as u16) << 8;
        match state.cpu.stage {
            Stage::OamDma(c) if c % 2 == 0 => {
                state.cpu.oam_dma_buffer = 
                    self.bus.read(system, state, ((c / 2) as u16) | high_addr);
                state.cpu.stage = state.cpu.stage.increment();
            },
            Stage::OamDma(c) if c % 2 == 1 => {
                let value = state.cpu.oam_dma_buffer;
                self.bus.write(system, state, 0x4014, value);
                if c == 511 {
                    state.cpu.stage = Stage::Fetch;
                } else {
                    state.cpu.stage = state.cpu.stage.increment();
                }
            },
            _ => unreachable!(),
        }
    }

    fn nmi(&self, system: &System, state: &mut SystemState) {
        match state.cpu.stage {
            Stage::Nmi(0) => {
                let addr = state.cpu.reg_pc as u16;
                self.bus.read(system, state, addr);
            },
            Stage::Nmi(1) => {
                let addr = state.cpu.reg_pc as u16;
                self.bus.read(system, state, addr);
            },
            Stage::Nmi(2) => {
                let val = (state.cpu.reg_pc >> 8) & 0xff;
                self.push_stack(system, state, val as u8);
            },
            Stage::Nmi(3) => {
                let val = state.cpu.reg_pc & 0xff;
                self.push_stack(system, state, val as u8);
            },
            Stage::Nmi(4) => {
                let val = state.cpu.reg_p() | 0x20;
                self.push_stack(system, state, val);
            },
            Stage::Nmi(5) => {
                let val = self.bus.read(system, state, 0xfffa);
                state.cpu.reg_pc &= 0xff00;
                state.cpu.reg_pc |= val as u32;
                state.cpu.flag_i = 1;
            },
            Stage::Nmi(6) => {
                let val = self.bus.read(system, state, 0xfffb) as u16;
                state.cpu.reg_pc &= 0x00ff;
                state.cpu.reg_pc |= (val << 8) as u32;
            }
            _ => unreachable!(),
        }
        state.cpu.stage = state.cpu.stage.increment();
    }

    fn decode(&self, system: &System, state: &mut SystemState) {
        if state.cpu.pending_oam_dma.is_some() {
            state.cpu.stage = Stage::OamDma(0);
            self.oam_dma(system, state);
        };
        let pc = state.cpu.reg_pc;
        let value = self.read_pc(system, state);
        print!("\n{:04X} {:X}  A:{:X} X:{:X} Y:{:X} P:{:X}", pc, value, state.cpu.reg_a
               , state.cpu.reg_x, state.cpu.reg_y, state.cpu.reg_p());
        state.cpu.op = self.ops[&value];
        state.cpu.stage = Stage::Address(0)
    }

    fn addressing(&self, system: &System, state: &mut SystemState) {
        let current = (state.cpu.op.addressing, state.cpu.stage);
        match current {
            (Addressing::None, Stage::Address(0)) => {
                let r = (state.cpu.reg_pc as u16).wrapping_add(1);
                let _ = self.bus.read(system, state, r);
            },
            (Addressing::Accumulator, Stage::Address(0)) => {
                let r = (state.cpu.reg_pc as u16).wrapping_add(1);
                let _ = self.bus.read(system, state, r);
                state.cpu.op_addr = state.cpu.reg_a as u16;      
            },
            (Addressing::Immediate, Stage::Address(0)) => {
                state.cpu.op_addr = state.cpu.reg_pc as u16;
                let add = state.cpu.op_addr;
                let value = self.bus.read(system, state, add);
                state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(1);
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
                let a = (state.cpu.op_addr).wrapping_add(state.cpu.reg_x as u16);
                state.cpu.op_addr = a & 0xff;
            },
            (Addressing::ZeroPageY, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::ZeroPageY, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = (state.cpu.op_addr).wrapping_add(state.cpu.reg_y as u16);
                state.cpu.op_addr = a & 0xff;
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
                if a & 0xff00 != a.wrapping_add(state.cpu.reg_x as u16) & 0xff {
                    let dummy_a = (a & 0xff00) |
                        (a.wrapping_add(state.cpu.reg_x as u16) & 0xff);
                    let _ = self.bus.read(system, state, dummy_a);
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(state.cpu.reg_x as u16);
                } else {
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(state.cpu.reg_x as u16);
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
                if a & 0xff00 != a.wrapping_add(state.cpu.reg_y as u16) & 0xff {
                    let dummy_a = (a & 0xff00) |
                        (a.wrapping_add(state.cpu.reg_y as u16) & 0xff);
                    let _ = self.bus.read(system, state, dummy_a); 
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(state.cpu.reg_y as u16);
                } else {
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(state.cpu.reg_y as u16);
                    let a = state.cpu.op_addr;
                    let v = self.bus.read(system, state, a);
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
                let dummy_a = (a & 0xff00) |
                    (a.wrapping_add(state.cpu.reg_x as u16) & 0xff);
                let _ = self.bus.read(system, state, dummy_a);
                state.cpu.op_addr = state.cpu.op_addr
                    .wrapping_add(state.cpu.reg_x as u16);
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
                let dummy_a = (a & 0xff00) |
                    (a.wrapping_add(state.cpu.reg_y as u16) & 0xff);
                let _ = self.bus.read(system, state, dummy_a);
                state.cpu.op_addr = state.cpu.op_addr
                    .wrapping_add(state.cpu.reg_y as u16);
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
                let a = (state.cpu.op_addr & 0xff00) |
                    (state.cpu.op_addr.wrapping_add(1) & 0xff);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a |
                    state.cpu.decode_stack.pop_back().unwrap() as u16;
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
                let a = a.wrapping_add(state.cpu.reg_x as u16);
                let a = a & 0xff;
                state.cpu.op_addr = a;
            },
            (Addressing::IndirectX, Stage::Address(2)) => {
                let a = state.cpu.op_addr;
                let v = self.bus.read(system, state, 0);
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
            },
            (Addressing::IndirectX, Stage::Address(3)) => {
                let a = (state.cpu.op_addr & 0xff00) |
                    (state.cpu.op_addr.wrapping_add(1) & 0xff);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a
                    | state.cpu.decode_stack.pop_back().unwrap() as u16;
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
                let a = (state.cpu.op_addr & 0xff00) |
                    (state.cpu.op_addr.wrapping_add(1) & 0xff);
                let a = self.bus.read(system, state, a);
                let a_low = state.cpu.decode_stack.pop_back().unwrap();
                state.cpu.decode_stack.push_back(a);
                state.cpu.decode_stack.push_back(a_low);
                state.cpu.op_addr = ((a as u16) << 8) | a_low as u16;
                state.cpu.op_addr = state.cpu.op_addr.
                    wrapping_add((state.cpu.reg_y & 0xff) as u16);
            },
            (Addressing::IndirectY, Stage::Address(3)) => {
                let low = state.cpu.decode_stack.pop_back().unwrap() as u16;
                let high = (state.cpu.decode_stack.pop_back().unwrap() as u16) << 8;
                let a = high | low;
                if high != (a.wrapping_add((state.cpu.reg_y & 0xff) as u16) & 0xff00) {
                    let a = high | (a.wrapping_add((state.cpu.reg_y & 0xff) as u16) & 0xff);
                    let _ = self.bus.read(system, state, a);
                } else {
                    state.cpu.stage = Stage::Execute(0);
                    self.operation(system, state);
                    return;
                }
            },
            (Addressing::IndirectYDummyAlways, Stage::Address(0)) => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
            },
            (Addressing::IndirectYDummyAlways, Stage::Address(1)) => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
            },
            (Addressing::IndirectYDummyAlways, Stage::Address(2)) => {
                let a = (state.cpu.op_addr & 0xff00) |
                    (state.cpu.op_addr.wrapping_add(1) & 0xff);
                let a = self.bus.read(system, state, a);
                let a_low = state.cpu.decode_stack.pop_back().unwrap();
                state.cpu.decode_stack.push_back(a);
                state.cpu.decode_stack.push_back(a_low);
                state.cpu.op_addr = ((a as u16) << 8) | a_low as u16;
                state.cpu.op_addr = state.cpu.op_addr.
                    wrapping_add((state.cpu.reg_y & 0xff) as u16);
            },
            (Addressing::IndirectYDummyAlways, Stage::Address(3)) => {
                let low = state.cpu.decode_stack.pop_back().unwrap() as u16;
                let high = (state.cpu.decode_stack.pop_back().unwrap() as u16) << 8;
                let a = high | low;
                let a = high | (a.wrapping_add((state.cpu.reg_y & 0xff) as u16) & 0xff);
                let _ = self.bus.read(system, state, a);
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
        let current = (state.cpu.op.instruction, state.cpu.stage);
        let addr = state.cpu.op_addr;
        match current {
            (Instruction::Adc, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                let temp = state.cpu.reg_a.wrapping_add(
                    value.wrapping_add(state.cpu.flag_c));
                state.cpu.flag_v = ((!(state.cpu.reg_a ^ value) &
                                 (state.cpu.reg_a ^ temp)) >> 7) & 1;
                state.cpu.flag_c  = if temp > 0xff { 1 } else { 0 };
                state.cpu.reg_a = temp & 0xff;
                state.cpu.flag_s = temp & 0xff;
                state.cpu.flag_z = temp & 0xff;
            },
            (Instruction::And, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32 & state.cpu.reg_a;
                state.cpu.reg_a = value;
                state.cpu.flag_s = value;
                state.cpu.flag_z = value;
            },
            (Instruction::Asl, Stage::Execute(0)) => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    state.cpu.flag_c = (state.cpu.reg_a >> 7) & 1;
                    state.cpu.reg_a = (state.cpu.reg_a << 1) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                }
            },
            (Instruction::Asl, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Asl, Stage::Execute(2)) => {
                let mut value = state.cpu.decode_stack.pop_back().unwrap() as u32;
                state.cpu.flag_c = (value >> 7) & 1;
                value = (value << 1) & 0xff;
                state.cpu.flag_z = value;
                state.cpu.flag_s = value;
                self.bus.write(system, state, addr, value as u8);
            },
            (Instruction::Bcc, Stage::Execute(0)) => {
                if state.cpu.flag_c == 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bcc, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Bcs, Stage::Execute(0)) => {
                if state.cpu.flag_c != 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bcs, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Beq, Stage::Execute(0)) => {
                if state.cpu.flag_z == 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Beq, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Bit, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_s = value & 0x80;
                state.cpu.flag_v = (value >> 6) & 1;
                state.cpu.flag_z = value & state.cpu.reg_a;
            },
            (Instruction::Bmi, Stage::Execute(0)) => {
                if state.cpu.flag_s & 0x80 != 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bmi, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Bne, Stage::Execute(0)) => {
                if state.cpu.flag_z != 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bne, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Bpl, Stage::Execute(0)) => {
                if state.cpu.flag_s & 0x80 == 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bpl, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Brk, Stage::Execute(0)) => {
                let _ = self.bus.read(system, state, addr);
            },
            (Instruction::Brk, Stage::Execute(1)) => {
                let value = state.cpu.reg_pc & 0xff;
                self.push_stack(system, state, value as u8);
            },
            (Instruction::Brk, Stage::Execute(2)) => {
                let value = state.cpu.reg_pc >> 8 & 0xff;
                self.push_stack(system, state, value as u8);
            },
            (Instruction::Brk, Stage::Execute(3)) => {
                let value = state.cpu.reg_p() | 0x30;
                self.push_stack(system, state, value);
                state.cpu.flag_i = 1;
            },
            (Instruction::Brk, Stage::Execute(4)) => {
                let value = self.bus.read(system, state, 0xfffe);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Brk, Stage::Execute(5)) => {
                let high_value = self.bus.read(system, state, 0xffff);
                let value = state.cpu.decode_stack.pop_back().unwrap();
                state.cpu.reg_pc = value as u32 | ((high_value as u32) <<  0x8);
            },
            (Instruction::Bvc, Stage::Execute(0)) => {
                if state.cpu.flag_v == 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bvc, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Bvs, Stage::Execute(0)) => {
                if state.cpu.flag_v != 0 {
                    let _ = self.bus.read(system, state, addr);
                } else {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                }
            },
            (Instruction::Bvs, Stage::Execute(1)) => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                } else {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32).wrapping_sub(256)
                             & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        state.cpu.stage = Stage::Fetch;
                        self.decode(system, state);
                        return;
                    }
                }
            },
            (Instruction::Clc, Stage::Execute(0)) => {
                state.cpu.flag_c = 0;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Cld, Stage::Execute(0)) => {
                state.cpu.flag_d = 0;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Cli, Stage::Execute(0)) => {
                state.cpu.flag_i = 0;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Clv, Stage::Execute(0)) => {
                state.cpu.flag_v = 0;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Cmp, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_c = if state.cpu.reg_a >= value { 1 } else { 0 };
                state.cpu.flag_z = if state.cpu.reg_a == value { 0 } else { 1 };
                state.cpu.flag_s = state.cpu.reg_a.wrapping_sub(value) & 0xff;
            },
            (Instruction::Cpx, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_c = if state.cpu.reg_x >= value { 1 } else { 0 };
                state.cpu.flag_z = if state.cpu.reg_x == value { 0 } else { 1 };
                state.cpu.flag_s = state.cpu.reg_x.wrapping_sub(value) & 0xff;
            },
            (Instruction::Cpy, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_c = if state.cpu.reg_y >= value { 1 } else { 0 };
                state.cpu.flag_z = if state.cpu.reg_y == value { 0 } else { 1 };
                state.cpu.flag_s = state.cpu.reg_y.wrapping_sub(value) & 0xff;
            },
            (Instruction::Dec, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Dec, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let value = value.wrapping_sub(1) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Dec, Stage::Execute(2)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
            },
            (Instruction::Dex, Stage::Execute(0)) => {
                state.cpu.reg_x = state.cpu.reg_x.wrapping_sub(1) & 0xff;
                state.cpu.flag_s = state.cpu.reg_x;
                state.cpu.flag_z = state.cpu.reg_x;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Dey, Stage::Execute(0)) => {
                state.cpu.reg_y = state.cpu.reg_y.wrapping_sub(1) & 0xff;
                state.cpu.flag_s = state.cpu.reg_y;
                state.cpu.flag_z = state.cpu.reg_y;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Eor, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as u32;
                state.cpu.reg_a ^= value;
                state.cpu.reg_a &= 0xff;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
            },
            (Instruction::Inc, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Inc, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let value = value.wrapping_add(1) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Inc, Stage::Execute(2)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
            },
            (Instruction::Inx, Stage::Execute(0)) => {
                state.cpu.reg_x = state.cpu.reg_x.wrapping_add(1) & 0xff;
                state.cpu.flag_s = state.cpu.reg_x;
                state.cpu.flag_z = state.cpu.reg_x;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Iny, Stage::Execute(0)) => {
                state.cpu.reg_y = state.cpu.reg_y.wrapping_add(1) & 0xff;
                state.cpu.flag_s = state.cpu.reg_y;
                state.cpu.flag_z = state.cpu.reg_y;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Jmp, Stage::Execute(0)) => {
                state.cpu.reg_pc = addr as u32;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Jsr, Stage::Execute(0)) => {
                let a = state.cpu.reg_sp | 0x100;
                self.bus.read(system, state, a as u16);
            },
            (Instruction::Jsr, Stage::Execute(1)) => {
                let value = (state.cpu.reg_pc.wrapping_sub(1) >> 8) & 0xff;
                self.push_stack(system, state, value as u8);
            },
            (Instruction::Jsr, Stage::Execute(2)) => {
                let value = state.cpu.reg_pc.wrapping_sub(1) & 0xff;
                self.push_stack(system, state, value as u8);
                state.cpu.reg_pc = addr as u32;
            },
            (Instruction::Lda, Stage::Execute(0)) => {
                state.cpu.reg_a = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
            },
            (Instruction::Ldx, Stage::Execute(0)) => {
                state.cpu.reg_x = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_s = state.cpu.reg_x;
                state.cpu.flag_z = state.cpu.reg_x;
            },
            (Instruction::Ldy, Stage::Execute(0)) => {
                state.cpu.reg_y = self.bus.read(system, state, addr) as u32;
                state.cpu.flag_s = state.cpu.reg_y;
                state.cpu.flag_z = state.cpu.reg_y;
            },
            (Instruction::Lsr, Stage::Execute(0)) => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    state.cpu.flag_c = state.cpu.reg_a & 1;
                    state.cpu.reg_a >>= 1;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                }
            },
            (Instruction::Lsr, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                state.cpu.flag_c = (value as u32) & 1;
                let value = value >> 1;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Lsr, Stage::Execute(2)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
            },
            (Instruction::Nop, Stage::Execute(0)) => {
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Ora, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr);
                state.cpu.reg_a = (state.cpu.reg_a | value as u32) & 0xff;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
            },
            (Instruction::Pha, Stage::Execute(0)) => {
                let value = state.cpu.reg_a;
                self.push_stack(system, state, value as u8);
            },
            (Instruction::Php, Stage::Execute(0)) => {
                let value = state.cpu.reg_p() as u8 | 0x30;
                self.push_stack(system, state, value);
            },
            (Instruction::Pla, Stage::Execute(0)) => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
            },
            (Instruction::Pla, Stage::Execute(1)) => {
                state.cpu.reg_a = self.pop_stack(system, state) as u32;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
            },
            (Instruction::Plp, Stage::Execute(0)) => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
            },
            (Instruction::Plp, Stage::Execute(1)) => {
                let value = self.pop_stack(system, state) as u32;
                state.cpu.set_reg_p(value);
            },
            (Instruction::Rol, Stage::Execute(0)) => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    let c = if state.cpu.flag_c != 0 { 1 } else { 0 };
                    state.cpu.flag_c = state.cpu.reg_a >> 7 & 1;
                    state.cpu.reg_a = (state.cpu.reg_a << 1 | c) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                }
            },
            (Instruction::Rol, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let c = if state.cpu.flag_c != 0 { 1 } else { 0 };
                state.cpu.flag_c = value as u32 >> 7 & 1;
                let value = (value << 1 | c) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Rol, Stage::Execute(2)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
            }, 
            (Instruction::Ror, Stage::Execute(0)) => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    let c = if state.cpu.flag_c != 0 { 0x80 } else { 0 };
                    state.cpu.flag_c = state.cpu.reg_a & 1;
                    state.cpu.reg_a = (state.cpu.reg_a >> 1 | c) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                    return;
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                }
            },
            (Instruction::Ror, Stage::Execute(1)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let c = if state.cpu.flag_c != 0 { 0x80 } else { 0 };
                state.cpu.flag_c = value as u32 & 1;
                let value = (value >> 1 | c) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Ror, Stage::Execute(2)) => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
            },
            (Instruction::Rti, Stage::Execute(0)) => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
            },
            (Instruction::Rti, Stage::Execute(1)) => {
                let value = self.pop_stack(system, state);
                state.cpu.set_reg_p(value as u32);
            },
            (Instruction::Rti, Stage::Execute(2)) => {
                let value = self.pop_stack(system, state);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Rti, Stage::Execute(3)) => {
                let high_value = (self.pop_stack(system, state) as u16) << 8;
                let value = state.cpu.decode_stack.pop_back().unwrap() as u16;
                state.cpu.reg_pc = (high_value | value) as u32;
            },
            (Instruction::Rts, Stage::Execute(0)) => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
            },
            (Instruction::Rts, Stage::Execute(1)) => {
                let value = self.pop_stack(system, state);
                state.cpu.decode_stack.push_back(value);
            },
            (Instruction::Rts, Stage::Execute(2)) => {
                let high_value = (self.pop_stack(system, state) as u16) << 8;
                let value = state.cpu.decode_stack.pop_back().unwrap() as u16;
                state.cpu.reg_pc = (high_value | value).wrapping_add(1) as u32;
            },
            (Instruction::Rts, Stage::Execute(3)) => {
                let a = state.cpu.reg_pc;
                let _ = self.bus.read(system, state, a as u16);
            },
            (Instruction::Sbc, Stage::Execute(0)) => {
                let value = self.bus.read(system, state, addr) as i32;
                let temp_a = state.cpu.reg_a as i32;
                let temp = temp_a.wrapping_sub(
                            value.wrapping_sub(state.cpu.flag_c as i32 - 1));
                state.cpu.flag_v = (((temp_a ^ value) &
                                 (temp_a ^ temp)) >> 7) as u32 & 1;
                state.cpu.flag_c  = if temp < 0 { 0 } else { 1 };
                state.cpu.reg_a = (temp as u32) & 0xff;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
            },
            (Instruction::Sec, Stage::Execute(0)) => {
                state.cpu.flag_c = 1;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Sed, Stage::Execute(0)) => {
                state.cpu.flag_d = 1;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Sei, Stage::Execute(0)) => {
                state.cpu.flag_i = 1;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Sta, Stage::Execute(0)) => {
                let value = state.cpu.reg_a;
                self.bus.write(system, state, addr, value as u8); 
            },
            (Instruction::Stx, Stage::Execute(0)) => {
                let value = state.cpu.reg_x;
                self.bus.write(system, state, addr, value as u8); 
            },
            (Instruction::Sty, Stage::Execute(0)) => {
                let value = state.cpu.reg_y;
                self.bus.write(system, state, addr, value as u8); 
            },
            (Instruction::Tax, Stage::Execute(0)) => {
                state.cpu.reg_x = state.cpu.reg_a;
                state.cpu.flag_s = state.cpu.reg_x;
                state.cpu.flag_z = state.cpu.reg_x;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Tay, Stage::Execute(0)) => {
                state.cpu.reg_y = state.cpu.reg_a;
                state.cpu.flag_s = state.cpu.reg_y;
                state.cpu.flag_z = state.cpu.reg_y;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Tsx, Stage::Execute(0)) => {
                state.cpu.reg_x = state.cpu.reg_sp;
                state.cpu.flag_s = state.cpu.reg_x;
                state.cpu.flag_z = state.cpu.reg_x;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Txa, Stage::Execute(0)) => {
                state.cpu.reg_a = state.cpu.reg_x;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Txs, Stage::Execute(0)) => {
                state.cpu.reg_sp = state.cpu.reg_x;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            (Instruction::Tya, Stage::Execute(0)) => {
                state.cpu.reg_a = state.cpu.reg_y;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            },
            _ => {
                state.cpu.stage = Stage::Fetch;
                self.decode(system, state);
                return;
            }
            
        }
        state.cpu.stage = current.1.increment();
    }

}

