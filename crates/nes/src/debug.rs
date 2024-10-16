#[cfg(feature = "debugger")]
pub use debugger::*;

#[cfg(not(feature = "debugger"))]
pub use no_debugger::*;

#[cfg(feature = "debugger")]
mod debugger {
    use std::cell::RefCell;

    use crate::bus::{AddressBus, DeviceKind, RangeAndMask};
    use crate::cpu::{ops::*, CpuDebugState};
    use crate::machine::BreakpointHandler;
    use crate::memory::MemoryBlock;
    use crate::ppu::PpuDebugState;
    use crate::Machine;

    const INST_HISTORY_BUF_SIZE: usize = 100;

    #[derive(Debug, Clone, Default)]
    pub struct MachineState {
        pub cpu: CpuDebugState,
        pub ppu: PpuDebugState,
    }

    pub struct DebugState {
        trace_instrs: u32,
        color_dots: u32,
        log_once: bool,
        log_range: Option<(u16, u16)>,
        logging_range: bool,
        inst_ring: RingBuffer<(CpuDebugState, PpuDebugState)>,
        trace_fn: Option<Box<dyn FnMut(CpuDebugState, PpuDebugState)>>,
        machine_state: MachineState,
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
                trace_fn: None,
                machine_state: MachineState::default(),
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

    pub struct Debug {
        state: RefCell<DebugState>,
        mem: Option<MemoryBlock>,
    }

    impl Debug {
        pub fn new() -> Self {
            Self {
                state: RefCell::new(DebugState::default()),
                mem: None,
            }
        }

        pub fn register(&mut self, bus: &mut AddressBus, addr: u16, size_kb: u16) {
            let end = addr + (size_kb * 0x400);
            let mask = size_kb * 0x400 - 1;
            self.mem = Some(MemoryBlock::new(size_kb as usize));
            bus.register_read(DeviceKind::Debug, RangeAndMask(addr, addr + end, mask));
            bus.register_write(DeviceKind::Debug, RangeAndMask(addr, addr + end, mask));
        }

        pub fn read(&self, addr: u16) -> u8 {
            if let Some(mem) = &self.mem {
                mem.read(addr)
            } else {
                0x00
            }
        }

        pub fn write(&self, addr: u16, value: u8) {
            if let Some(mem) = &self.mem {
                mem.write(addr, value);
            }
        }

        pub fn frame(&self, system: &Machine) -> u32 {
            system.ppu.frame()
        }

        pub fn color_for(&self, count: u32) {
            self.state.borrow_mut().color_dots = count;
        }

        pub fn color(&self) -> bool {
            let mut state = self.state.borrow_mut();
            if state.color_dots != 0 {
                state.color_dots -= 1;
                true
            } else {
                false
            }
        }

        pub fn log_once_for(&self, count: u32) {
            let mut state = self.state.borrow_mut();
            if !state.log_once {
                state.log_once = true;
                state.trace_instrs = count;
            }
        }
        pub fn log_for(&self, count: u32) {
            let mut state = self.state.borrow_mut();
            state.trace_instrs = count;
        }

        pub fn log_range(&self, start_addr: u16, end_addr: u16) {
            let mut state = self.state.borrow_mut();
            state.log_range = Some((start_addr, end_addr));
        }

        pub fn log_history(&self, system: &Machine) {
            let state = self.state.borrow();
            self.do_log_history(system, &*state)
        }

        fn do_log_history(&self, system: &Machine, state: &DebugState) {
            for (cpu_state, ppu_state) in state.log_iter().rev() {
                let log = self.trace_instruction(system, cpu_state, ppu_state);
                tracing::info!("{}", log);
            }
        }

        pub fn trace(&self, system: &Machine, cpu: CpuDebugState) {
            let mut state = self.state.borrow_mut();
            state.machine_state.cpu = cpu;
            let ppu = state.machine_state.ppu;
            if let Some(inst_addr) = cpu.instruction_addr {
                state.logging_range = if let Some(&(start, end)) = state.log_range.as_ref() {
                    if inst_addr >= start && inst_addr <= end {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if state.trace_instrs != 0 || state.logging_range {
                    let log = self.trace_instruction(system, &cpu, &ppu);
                    tracing::info!("{}", log);
                    if state.trace_instrs != 0 {
                        state.trace_instrs -= 1;
                    }
                }

                state.log_inst((cpu, ppu));

                let inst = OPS[system.peek(inst_addr) as usize];
                if let Instruction::IllKil = inst.instruction {
                    self.do_log_history(system, &*state);
                }
            }

            if let Some(trace_fn) = state.trace_fn.as_mut() {
                trace_fn(cpu, ppu);
            }
        }

        pub fn trace_ppu(&self, _system: &Machine, ppu: PpuDebugState) {
            let mut state = self.state.borrow_mut();
            state.machine_state.ppu = ppu;
            let cpu = state.machine_state.cpu;

            if let Some(trace_fn) = state.trace_fn.as_mut() {
                trace_fn(cpu, ppu);
            }
        }

        pub fn trace_instruction(
            &self,
            system: &Machine,
            cpu_state: &CpuDebugState,
            ppu_state: &PpuDebugState,
        ) -> String {
            let addr = cpu_state.instruction_addr.unwrap();
            let instr = system.peek(addr);

            let op = OPS[instr as usize];
            let name = op.instruction.name();
            let len = op.addressing.length() as u16;

            let mut addr_inc = addr + 1;
            let read = |addr| -> u8 { system.peek(addr) };

            let mut read_pc = || -> u8 {
                let val = system.peek(addr_inc);
                addr_inc = addr_inc.wrapping_add(1);
                val
            };

            let pc_string = format!("{:04X}", addr);

            let instr_bytes_string = {
                let mut buf = String::new();
                for x in 0..len {
                    buf.push_str(&*format!(" {:02X}", read(x.wrapping_add(addr))));
                }

                buf
            };

            let addr_string = match op.addressing {
                Addressing::None => format!(""),
                Addressing::Accumulator => format!("A = {:02X}", cpu_state.reg_a),
                Addressing::Immediate => {
                    let r = read_pc();
                    format!("#${:02X}", r)
                }
                Addressing::ZeroPage(..) => {
                    let a = read_pc();
                    let v = read(a as u16);
                    format!("${:02X} = {:02X}", a, v)
                }
                Addressing::ZeroPageOffset(Reg::X, ..) => {
                    let a1 = read_pc() as u16;
                    let a2 = a1.wrapping_add(cpu_state.reg_x as u16) & 0xff;
                    let v = read(a2);
                    format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
                }
                Addressing::ZeroPageOffset(Reg::Y, ..) => {
                    let a1 = read_pc() as u16;
                    let a2 = a1.wrapping_add(cpu_state.reg_y as u16) & 0xff;
                    let v = read(a2);
                    format!("${:02X},X @ {:04X} = {:02X}", a1, a2, v)
                }
                Addressing::Absolute(..) => {
                    let a_low = read_pc() as u16;
                    let a_high = read_pc();
                    let a = ((a_high as u16) << 8) | a_low;
                    let v = read(a);
                    format!("${:04X} = ${:02X}", a, v)
                }
                Addressing::AbsoluteOffset(Reg::X, ..) => {
                    let a_low = read_pc() as u16;
                    let a_high = read_pc();
                    let a1 = ((a_high as u16) << 8) | a_low;
                    let a2 = a1.wrapping_add(cpu_state.reg_x as u16);
                    let v = read(a2);
                    format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
                }
                Addressing::AbsoluteOffset(Reg::Y, ..) => {
                    let a_low = read_pc() as u16;
                    let a_high = read_pc();
                    let a1 = ((a_high as u16) << 8) | a_low;
                    let a2 = a1.wrapping_add(cpu_state.reg_y as u16);
                    let v = read(a2);
                    format!("${:04X},X @ {:04X} = {:02X}", a1, a2, v)
                }
                Addressing::IndirectAbsolute(..) => {
                    let a1_low = read_pc() as u16;
                    let a1_high = read_pc();
                    let a1 = ((a1_high as u16) << 8) | a1_low;
                    let a2_low = read(a1) as u16;
                    let a2_high = read((a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                    let a2 = ((a2_high as u16) << 8) | a2_low;
                    format!("(${:04X}) @ {:04X}", a1, a2)
                }
                Addressing::Relative(..) => {
                    let rel = read_pc();
                    let a = if rel < 0x080 {
                        addr_inc.wrapping_add(rel as u16)
                    } else {
                        addr_inc.wrapping_sub(256).wrapping_add(rel as u16)
                    };
                    format!("${:04X}", a)
                }
                Addressing::IndirectX(..) => {
                    let a1 = read_pc() as u16;
                    let a2 = a1.wrapping_add(cpu_state.reg_x as u16) & 0xff;
                    let a3_low = read(a2) as u16;
                    let a3_high = read((a2 & 0xff00) | (a2.wrapping_add(1) & 0xff)) as u16;
                    let a3 = (a3_high << 8) | a3_low;
                    let v = read(a3);
                    format!("(${:02X},X) @ {:02X} = {:04X} = {:02X}", a1, a2, a3, v)
                }
                Addressing::IndirectY(..) => {
                    let a1 = read_pc() as u16;
                    let a2_low = read(a1);
                    let a2_high = read((a1 & 0xff00) | (a1.wrapping_add(1) & 0xff));
                    let a2 = ((a2_high as u16) << 8) | a2_low as u16;
                    let a3 = a2.wrapping_add(cpu_state.reg_y as u16);
                    let v = read(a3);
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
                    "DOT: {:3} SL: {:3} TICK: {:9} S0:{}",
                    ppu_state.dot,
                    ppu_state.scanline,
                    ppu_state.tick,
                    if ppu_state.sprite_zero_hit { 1 } else { 0 }
                )
            };

            format!(
                "{}{: <10.10}{: >4.4} {: <30.30} {} {}",
                pc_string, instr_bytes_string, name, addr_string, reg_string, ppu_string
            )
        }

        pub fn trace_fn<F: FnMut(CpuDebugState, PpuDebugState) -> () + 'static>(
            &self,
            trace_fn: F,
        ) {
            let mut state = self.state.borrow_mut();
            state.trace_fn = Some(Box::new(trace_fn));
        }

        pub fn pallete_ram<'m>(&self, machine: &'m Machine) -> &'m [u8] {
            machine.ppu.palette_data.as_slice()
        }

        pub fn sprite_ram<'m>(&self, machine: &'m Machine) -> &'m [u8] {
            machine.ppu.oam_data.as_slice()
        }

        pub fn machine_state(&self) -> MachineState {
            let state = self.state.borrow();
            state.machine_state.clone()
        }

        pub fn breakpoint<H: BreakpointHandler>(&self, handler: &H) -> bool {
            let state = self.state.borrow();
            handler.breakpoint(&state.machine_state)
        }
    }
}

#[cfg(not(feature = "debugger"))]
pub mod no_debugger {
    use crate::cpu::CpuDebugState;
    use crate::machine::BreakpointHandler;
    use crate::ppu::PpuDebugState;
    use crate::Machine;
    pub struct Debug;

    #[derive(Debug, Clone, Default)]
    pub struct MachineState;

    impl Debug {
        pub fn new() -> Self {
            Debug
        }

        pub fn read(&self, _addr: u16) -> u8 {
            0
        }

        pub fn write(&self, _addr: u16, _value: u8) {}

        pub fn trace(&self, _system: &Machine, _cpu_state: CpuDebugState) {}

        pub fn trace_ppu(&self, _system: &Machine, _ppu_state: PpuDebugState) {}

        pub fn breakpoint<H: BreakpointHandler>(&self, _handler: &H) -> bool {
            false
        }
    }
}
