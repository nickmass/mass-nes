use crate::cpu::CpuDebugState;
use crate::ops::*;
use crate::ppu::PpuDebugState;
use crate::system::{System, SystemState};

use std::collections::VecDeque;

#[derive(Default)]
pub struct DebugState {
    trace_instrs: u32,
    color_dots: u32,
    log_once: bool,
    inst_queue: VecDeque<(CpuDebugState, PpuDebugState)>,
}

pub struct Debug {
    ops: &'static [Op; 0x100],
}

impl Debug {
    pub fn new() -> Debug {
        Debug { ops: Op::load() }
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

    pub fn log_history(&self, system: &System, state: &mut SystemState) {
        let queue = state.debug.inst_queue.clone();
        for &(cpu_state, ppu_state) in queue.iter() {
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
            if state.debug.inst_queue.len() == 100 {
                let _ = state.debug.inst_queue.pop_front();
            }
            state.debug.inst_queue.push_back((cpu_state, ppu_state));
            if state.debug.trace_instrs != 0 {
                let log = self.trace_instruction(system, state, cpu_state, ppu_state);
                eprintln!("{}", log);
                state.debug.trace_instrs -= 1;
            }

            let inst = self.ops[self.peek(system, state, inst_addr) as usize];
            if let Instruction::IllKil = inst.instruction {
                self.log_history(system, state);
                panic!("KIL");
            }
        }
    }

    pub fn trace_instruction(
        &self,
        system: &System,
        state: &mut SystemState,
        cpu_state: CpuDebugState,
        ppu_state: PpuDebugState,
    ) -> String {
        let addr = cpu_state.instruction_addr.unwrap();
        let instr = self.peek(system, state, addr);

        let op = self.ops[instr as usize];
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
                let a3 = a2.wrapping_add((cpu_state.reg_y & 0xff) as u16);
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
