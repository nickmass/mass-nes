use crate::cpu::CpuDebugState;
use crate::ops::*;
use crate::ppu::PpuDebugState;
use crate::system::{System, SystemState};

const INST_HISTORY_BUF_SIZE: usize = 100;

pub struct DebugState {
    trace_instrs: u32,
    color_dots: u32,
    log_once: bool,
    log_range: Option<(u16, u16)>,
    pub logging_range: bool,
    inst_ring: RingBuffer<(CpuDebugState, PpuDebugState)>,
}

impl Default for DebugState {
    fn default() -> DebugState {
        DebugState {
            trace_instrs: 0,
            color_dots: 0,
            log_once: false,
            log_range: None,
            logging_range: false,
            inst_ring: RingBuffer::new(INST_HISTORY_BUF_SIZE),
        }
    }
}

impl DebugState {
    fn log_inst(&mut self, inst: (CpuDebugState, PpuDebugState)) {
        self.inst_ring.push(inst);
    }

    fn log_iter(&self) -> RingIter<(CpuDebugState, PpuDebugState)> {
        self.inst_ring.iter()
    }
}

struct RingBuffer<T> {
    ring: Box<[Option<T>]>,
    ring_index: usize,
}

impl<T> RingBuffer<T> {
    fn new(capacity: usize) -> RingBuffer<T> {
        let mut ring = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            ring.push(None);
        }

        RingBuffer {
            ring: ring.into_boxed_slice(),
            ring_index: 0,
        }
    }

    fn push(&mut self, item: T) {
        self.ring_index += 1;
        if self.ring_index >= self.ring.len() {
            self.ring_index = 0;
        }

        self.ring[self.ring_index] = Some(item);
    }

    fn iter(&self) -> RingIter<T> {
        RingIter {
            ring: &self.ring[..],
            ring_end_index: self.ring_index,
            ring_index: self.ring_index,
            first: true,
        }
    }
}

struct RingIter<'a, T> {
    ring: &'a [Option<T>],
    ring_end_index: usize,
    ring_index: usize,
    first: bool,
}

impl<'a, T> Iterator for RingIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.first && self.ring_index == self.ring_end_index {
            None
        } else {
            let res = self.ring[self.ring_index].as_ref();
            if self.ring_index == 0 {
                self.ring_index = self.ring.len() - 1;
            } else {
                self.ring_index -= 1;
            }

            self.first = false;
            res
        }
    }
}

impl<'a, T> DoubleEndedIterator for RingIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if !self.first && self.ring_end_index == self.ring_index {
                return None;
            } else {
                self.ring_end_index += 1;
                if self.ring_end_index >= self.ring.len() {
                    self.ring_end_index = 0;
                }

                let res = self.ring[self.ring_end_index].as_ref();

                if res.is_some() {
                    self.first = false;
                    return res;
                }
            }
        }
    }
}

pub struct Debug;

impl Debug {
    pub fn new() -> Debug {
        Debug
    }

    pub fn peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        system.cpu_bus.peek(system, state, addr)
    }

    pub fn frame(&self, state: &SystemState) -> u32 {
        state.ppu.frame
    }

    pub fn color_for(&self, state: &mut SystemState, count: u32) {
        state.debug.color_dots = count;
    }

    pub fn color(&self, state: &mut SystemState) -> bool {
        if state.debug.color_dots != 0 {
            state.debug.color_dots -= 1;
            true
        } else {
            false
        }
    }

    pub fn log_once_for(&self, state: &mut SystemState, count: u32) {
        if !state.debug.log_once {
            state.debug.log_once = true;
            self.log_for(state, count);
        }
    }
    pub fn log_for(&self, state: &mut SystemState, count: u32) {
        state.debug.trace_instrs = count;
    }

    pub fn log_range(&self, state: &mut SystemState, start_addr: u16, end_addr: u16) {
        state.debug.log_range = Some((start_addr, end_addr));
    }

    pub fn log_history(&self, system: &System, state: &SystemState) {
        for (cpu_state, ppu_state) in state.debug.log_iter().rev() {
            let log = self.trace_instruction(system, state, cpu_state, ppu_state);
            eprintln!("{}", log);
        }
    }

    pub fn trace(
        &self,
        system: &System,
        state: &mut SystemState,
        cpu_state: CpuDebugState,
        ppu_state: PpuDebugState,
    ) {
        if let Some(inst_addr) = cpu_state.instruction_addr {
            if Some(inst_addr) == state.debug.log_range.as_ref().map(|r| r.0) {
                state.debug.logging_range = true;
            }
            if state.debug.trace_instrs != 0 || state.debug.logging_range {
                let log = self.trace_instruction(system, state, &cpu_state, &ppu_state);
                eprintln!("{}", log);
                if state.debug.trace_instrs != 0 {
                    state.debug.trace_instrs -= 1;
                }
            }
            if Some(inst_addr) == state.debug.log_range.as_ref().map(|r| r.1) {
                state.debug.logging_range = false;
            }

            state.debug.log_inst((cpu_state, ppu_state));

            let inst = super::ops::OPS[self.peek(system, state, inst_addr) as usize];
            if let Instruction::IllKil = inst.instruction {
                self.log_history(system, state);
                panic!("KIL");
            }
        }
    }

    pub fn trace_ppu(
        &self,
        system: &System,
        state: &mut SystemState,
        cpu_state: CpuDebugState,
        ppu_state: PpuDebugState,
    ) {
        if state.debug.logging_range {
            eprintln!("{:?}", ppu_state);
        }
    }

    pub fn trace_instruction(
        &self,
        system: &System,
        state: &SystemState,
        cpu_state: &CpuDebugState,
        ppu_state: &PpuDebugState,
    ) -> String {
        let addr = cpu_state.instruction_addr.unwrap();
        let instr = self.peek(system, state, addr);

        let op = super::ops::OPS[instr as usize];
        let name = op.instruction.name();
        let len = op.addressing.length() as u16;

        let mut addr_inc = addr + 1;
        let read = |state, addr| -> u8 { self.peek(system, state, addr) };

        let mut read_pc = |state| -> u8 {
            let val = self.peek(system, state, addr_inc);
            addr_inc = addr_inc.wrapping_add(1);
            val
        };

        let pc_string = format!("{:04X}", addr);

        let instr_bytes_string = {
            let mut buf = String::new();
            for x in 0..len {
                buf.push_str(&*format!(" {:02X}", read(state, x.wrapping_add(addr))));
            }

            buf
        };

        let addr_string = match op.addressing {
            Addressing::None => format!(""),
            Addressing::Accumulator => format!("A = {:02X}", cpu_state.reg_a),
            Addressing::Immediate => {
                let r = read_pc(state);
                format!("#${:02X}", r)
            }
            Addressing::ZeroPage(..) => {
                let a = read_pc(state);
                let v = read(state, a as u16);
                format!("${:02X} = {:02X}", a, v)
            }
            Addressing::ZeroPageOffset(Reg::X, ..) => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(cpu_state.reg_x as u16) & 0xff;
                let v = read(state, a2);
                format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
            }
            Addressing::ZeroPageOffset(Reg::Y, ..) => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(cpu_state.reg_y as u16) & 0xff;
                let v = read(state, a2);
                format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
            }
            Addressing::Absolute(..) => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a = ((a_high as u16) << 8) | a_low;
                let v = read(state, a);
                format!("${:04X} = ${:02X}", a, v)
            }
            Addressing::AbsoluteOffset(Reg::X, ..) => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a1 = ((a_high as u16) << 8) | a_low;
                let a2 = a1.wrapping_add(cpu_state.reg_x as u16);
                let v = read(state, a2);
                format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
            }
            Addressing::AbsoluteOffset(Reg::Y, ..) => {
                let a_low = read_pc(state) as u16;
                let a_high = read_pc(state);
                let a1 = ((a_high as u16) << 8) | a_low;
                let a2 = a1.wrapping_add(cpu_state.reg_y as u16);
                let v = read(state, a2);
                format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
            }
            Addressing::IndirectAbsolute(..) => {
                let a1_low = read_pc(state) as u16;
                let a1_high = read_pc(state);
                let a1 = ((a1_high as u16) << 8) | a1_low;
                let a2_low = read(state, a1) as u16;
                let a2_high = read(state, (a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                let a2 = ((a2_high as u16) << 8) | a2_low;
                format!("(${:04X}) @ {:04X}", a1, a2)
            }
            Addressing::Relative(..) => {
                let rel = read_pc(state);
                let a = if rel < 0x080 {
                    addr_inc.wrapping_add(rel as u16)
                } else {
                    addr_inc.wrapping_sub(256).wrapping_add(rel as u16)
                };
                format!("${:04X}", a)
            }
            Addressing::IndirectX(..) => {
                let a1 = read_pc(state) as u16;
                let a2 = a1.wrapping_add(cpu_state.reg_x as u16) & 0xff;
                let a3_low = read(state, a2) as u16;
                let a3_high = read(state, (a2 & 0xff00) | (a2.wrapping_add(1) & 0xff)) as u16;
                let a3 = (a3_high << 8) | a3_low;
                let v = read(state, a3);
                format!("(${:02X},X) @ {:02X} = {:04X} = {:02X}", a1, a2, a3, v)
            }
            Addressing::IndirectY(..) => {
                let a1 = read_pc(state) as u16;
                let a2_low = read(state, a1);
                let a2_high = read(state, (a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                let a2 = ((a2_high as u16) << 8) | a2_low as u16;
                let a3 = a2.wrapping_add(cpu_state.reg_y as u16);
                let v = read(state, a3);
                format!("(${:02X}),Y = {:04X} @ {:04X} = {:02X}", a1, a2, a3, v)
            }
        };

        let reg_string = {
            format!(
                "A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{:9}",
                cpu_state.reg_a,
                cpu_state.reg_x,
                cpu_state.reg_y,
                cpu_state.reg_p,
                cpu_state.reg_sp,
                cpu_state.cycle,
            )
        };

        let ppu_string = {
            format!(
                "DOT: {:3} SL: {:3} TICK: {:9}",
                ppu_state.dot, ppu_state.scanline, ppu_state.tick
            )
        };

        format!(
            "{}{: <10.10}{: >4.4} {: <30.30} {} {}",
            pc_string, instr_bytes_string, name, addr_string, reg_string, ppu_string
        )
    }
}
