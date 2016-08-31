use nes::bus::{BusKind, DeviceKind, AddressBus, AddressValidator};
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
    pub reg_a: u32,
    pub reg_x: u32,
    pub reg_y: u32,
    pub reg_pc: u32,
    pub reg_sp: u32,
    flag_c: u32,
    flag_z: u32,
    flag_i: u32,
    flag_d: u32,
    flag_v: u32,
    flag_s: u32,
    pending_oam_dma: Option<u8>,
    oam_dma_buffer: u8,
    oam_dma_addr: u16,

    pending_nmi: Option<u32>,
}

impl CpuState {
    pub fn reg_p(&self) -> u8 {
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

    pub fn nmi_req(&mut self, delay: u32) {
        self.pending_nmi = Some(delay);
    }

    pub fn nmi_cancel(&mut self) {
        self.pending_nmi = None;
    }
}

#[derive(Copy, Clone)]
enum StageResult {
    Continue,
    Done,
    Next,
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
            Stage::OamDma(n) => {
                if n == 511 {
                    Stage::Fetch
                }  else {
                    Stage::OamDma(n + 1)
                }
            },
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
    pub bus: AddressBus,
    pub mem: MemoryBlock,
    ops: HashMap<u8, Op>,
}

impl Cpu {
    pub fn new(state: &mut SystemState) -> Cpu {
        Cpu {
            bus: AddressBus::new(BusKind::Cpu, state, 0),
            mem: MemoryBlock::new(2, &mut state.mem),
            ops: Op::load(),
        }
    }
    
    pub fn power(&self, system: &System, state: &mut SystemState) {
        state.cpu.reg_pc = self.bus.read_word(system, state, 0xfffc) as u32;
        state.cpu.set_reg_p(0x34);
        state.cpu.reg_sp = 0xfd;
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
        match state.cpu.stage {
            Stage::OamDma(c) if c % 2 == 0 => {
                let high_addr = state.cpu.oam_dma_addr;
                state.cpu.oam_dma_buffer = 
                    self.bus.read(system, state, ((c / 2) as u16) | high_addr);
                state.cpu.stage = state.cpu.stage.increment();
            },
            Stage::OamDma(c) if c % 2 == 1 => {
                let value = state.cpu.oam_dma_buffer;
                self.bus.write(system, state, 0x2004, value);
                state.cpu.stage = state.cpu.stage.increment();
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
        if state.cpu.pending_nmi == Some(0) {
            state.cpu.stage = Stage::Nmi(0);
            state.cpu.pending_nmi = None;
            self.nmi(system, state);
            return;
        } else if state.cpu.pending_nmi.is_some() {
            let val = state.cpu.pending_nmi.unwrap();
            state.cpu.pending_nmi = Some(val - 1);
        }
        if state.cpu.pending_oam_dma.is_some() {
            state.cpu.oam_dma_addr = (state.cpu.pending_oam_dma.unwrap() as u16) << 8;
            state.cpu.pending_oam_dma = None;
            state.cpu.stage = Stage::OamDma(0);
            self.oam_dma(system, state);
            return;
        };
        let pc = state.cpu.reg_pc;
        let value = self.read_pc(system, state);
        system.debug.trace(system, state, pc as u16);
        state.cpu.op = self.ops[&value];
        state.cpu.stage = Stage::Address(0)
    }

    fn addressing(&self, system: &System, state: &mut SystemState) {
        if let Stage::Address(stage) = state.cpu.stage {
            let res = match state.cpu.op.addressing {
                Addressing::None => self.addr_none(system, state),
                Addressing::Accumulator => self.addr_accumulator(system, state),
                Addressing::Immediate => self.addr_immediate(system, state),
                Addressing::ZeroPage => self.addr_zero_page(system, state), 
                Addressing::ZeroPageX => {
                    let reg = state.cpu.reg_x;
                    self.addr_zero_page_offset(system, state, reg, stage)
                },
                Addressing::ZeroPageY => {
                    let reg = state.cpu.reg_y;
                    self.addr_zero_page_offset(system, state, reg, stage)
                },
                Addressing::Absolute => self.addr_absolute(system, state, stage),
                Addressing::AbsoluteX(d) => {
                    let reg = state.cpu.reg_x;
                    self.addr_absolute_offset(system, state, reg, stage, d)
                },
                Addressing::AbsoluteY(d) => {
                    let reg = state.cpu.reg_y;
                    self.addr_absolute_offset(system, state, reg, stage, d)
                },
                Addressing::IndirectAbsolute =>
                    self.addr_indirect_absolute(system, state, stage),
                Addressing::Relative => self.addr_relative(system, state), 
                Addressing::IndirectX => self.addr_indirect_x(system, state, stage),
                Addressing::IndirectY(d) =>
                    self.addr_indirect_y(system, state, stage, d),
            };
            
            match res {
                StageResult::Continue => {
                    state.cpu.stage = state.cpu.stage.increment();
                },
                StageResult::Done => {
                    state.cpu.stage = Stage::Execute(0);
                },
                StageResult::Next => {
                    state.cpu.stage = Stage::Execute(0);
                    self.operation(system, state);
                }
            }
        } else {
            unreachable!();
        }
    }

    fn addr_none(&self, system: &System, state: &mut SystemState) -> StageResult {
        let r = (state.cpu.reg_pc as u16).wrapping_add(1);
        let _ = self.bus.read(system, state, r);
        StageResult::Done
    }

    fn addr_accumulator(&self, system: &System, state: &mut SystemState)
    -> StageResult { 
        let r = (state.cpu.reg_pc as u16).wrapping_add(1);
        let _ = self.bus.read(system, state, r);
        state.cpu.op_addr = state.cpu.reg_a as u16;      
        StageResult::Done
    }

    fn addr_immediate(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.op_addr = state.cpu.reg_pc as u16;
        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(1);
        StageResult::Next
    }

    fn addr_zero_page(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        let a = self.read_pc(system, state);
        state.cpu.op_addr = a as u16;
        StageResult::Done
    }

    fn addr_zero_page_offset(&self, system: &System, state: &mut SystemState,
    reg: u32, stage: u32) -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = state.cpu.op_addr.wrapping_add(reg as u16);
                state.cpu.op_addr = a & 0xff;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn addr_absolute(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn addr_absolute_offset(&self, system: &System, state: &mut SystemState, 
    reg: u32, stage: u32, dummy: DummyRead) -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
                StageResult::Continue
            },
            2 => {
                let a = state.cpu.op_addr;
                if Self::will_wrap(a, (reg & 0xff) as u16) ||
                    dummy == DummyRead::Always {
                    let a =Self::wrapping_add(a, (reg & 0xff) as u16);
                    let _ = self.bus.read(system, state, a);
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(reg as u16);
                    StageResult::Done
                } else {
                    state.cpu.op_addr = state.cpu.op_addr
                        .wrapping_add(reg as u16);
                    StageResult::Next
                }
            },
            _ => unreachable!()
        }
    }

    fn addr_indirect_absolute(&self, system: &System, state: &mut SystemState,
    stage: u32) -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = self.read_pc(system, state);
                let a = ((a as u16) << 8) | state.cpu.op_addr;
                state.cpu.op_addr = a;
                StageResult::Continue
            },
            2 => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
                StageResult::Continue
            },
            3 => {
                let a = Self::wrapping_add(state.cpu.op_addr, 1);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a |
                    state.cpu.decode_stack.pop_back().unwrap() as u16;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn addr_relative(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        let a = self.read_pc(system, state);
        state.cpu.op_addr = a as u16;
        StageResult::Done
    }

    fn addr_indirect_x(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = state.cpu.op_addr;
                let _ = self.bus.read(system, state, a);
                let a = a.wrapping_add(state.cpu.reg_x as u16);
                let a = a & 0xff;
                state.cpu.op_addr = a;
                StageResult::Continue
            },
            2 => {
                let a = state.cpu.op_addr;
                let v = self.bus.read(system, state, 0);
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
                StageResult::Continue
            },
            3 => {
                let a = Self::wrapping_add(state.cpu.op_addr, 1);
                let a = (self.bus.read(system, state, a) as u16) << 8;
                state.cpu.op_addr = a
                    | state.cpu.decode_stack.pop_back().unwrap() as u16;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn addr_indirect_y(&self, system: &System, state: &mut SystemState,
    stage: u32, dummy: DummyRead) -> StageResult {
        match stage {
            0 => {
                let a = self.read_pc(system, state);
                state.cpu.op_addr = a as u16;
                StageResult::Continue
            },
            1 => {
                let a = state.cpu.op_addr;
                let a = self.bus.read(system, state, a);
                state.cpu.decode_stack.push_back(a);
                StageResult::Continue
            },
            2 => {
                let a = Self::wrapping_add(state.cpu.op_addr, 1);
                let a = self.bus.read(system, state, a);
                let a_low = state.cpu.decode_stack.pop_back().unwrap();
                state.cpu.decode_stack.push_back(a);
                state.cpu.decode_stack.push_back(a_low);
                state.cpu.op_addr = ((a as u16) << 8) | a_low as u16;
                state.cpu.op_addr = state.cpu.op_addr.
                    wrapping_add((state.cpu.reg_y & 0xff) as u16);
                StageResult::Continue
            },
            3 => {
                let low = state.cpu.decode_stack.pop_back().unwrap() as u16;
                let high = (state.cpu.decode_stack.pop_back().unwrap() as u16) << 8;
                let a = high | low;
                if Self::will_wrap(a, (state.cpu.reg_y & 0xff) as u16) ||
                    dummy == DummyRead::Always {
                    let a = Self::wrapping_add(a, (state.cpu.reg_y & 0xff) as u16);
                    let _ = self.bus.read(system, state, a);
                    StageResult::Done
                } else {
                    StageResult::Next
                }
            },
            _ => unreachable!()
        }
    }

    fn operation(&self, system: &System, state: &mut SystemState) {
        let addr = state.cpu.op_addr;
        if let Stage::Execute(stage) = state.cpu.stage {
            let res = match state.cpu.op.instruction {
                Instruction::Adc => self.inst_adc(system, state, addr),
                Instruction::And => self.inst_and(system, state, addr),
                Instruction::Asl => self.inst_asl(system, state, addr, stage),
                Instruction::Bcc => {
                    let cond = state.cpu.flag_c == 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Bcs => {
                    let cond = state.cpu.flag_c != 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Beq => {
                    let cond = state.cpu.flag_z == 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Bit => self.inst_bit(system, state, addr),
                Instruction::Bmi => {
                    let cond = state.cpu.flag_s & 0x80 != 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Bne => {
                    let cond = state.cpu.flag_z != 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Bpl => {
                    let cond = state.cpu.flag_s & 0x80 == 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Brk => self.inst_brk(system, state, addr, stage),
                Instruction::Bvc => {
                    let cond = state.cpu.flag_v == 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Bvs => {
                    let cond = state.cpu.flag_v != 0;
                    self.inst_branch(system, state, addr, stage, cond)
                },
                Instruction::Clc => self.inst_clc(system, state),
                Instruction::Cld => self.inst_cld(system, state),
                Instruction::Cli => self.inst_cli(system, state),
                Instruction::Clv => self.inst_clv(system, state), 
                Instruction::Cmp => self.inst_cmp(system, state, addr),
                Instruction::Cpx => self.inst_cpx(system, state, addr),
                Instruction::Cpy => self.inst_cpy(system, state, addr), 
                Instruction::Dec => self.inst_dec(system, state, addr, stage), 
                Instruction::Dex => self.inst_dex(system, state),
                Instruction::Dey => self.inst_dey(system, state), 
                Instruction::Eor => self.inst_eor(system, state, addr),
                Instruction::Inc => self.inst_inc(system, state, addr, stage), 
                Instruction::Inx => self.inst_inx(system, state),
                Instruction::Iny => self.inst_iny(system, state), 
                Instruction::Jmp => self.inst_jmp(system, state, addr), 
                Instruction::Jsr => self.inst_jsr(system, state, addr, stage),
                Instruction::Lda => self.inst_lda(system, state, addr), 
                Instruction::Ldx => self.inst_ldx(system, state, addr), 
                Instruction::Ldy => self.inst_ldy(system, state, addr), 
                Instruction::Lsr => self.inst_lsr(system, state, addr, stage),  
                Instruction::Nop => self.inst_nop(system, state),
                Instruction::Ora => self.inst_ora(system, state, addr), 
                Instruction::Pha => self.inst_pha(system, state), 
                Instruction::Php => self.inst_php(system, state), 
                Instruction::Pla => self.inst_pla(system, state, stage), 
                Instruction::Plp => self.inst_plp(system, state, stage), 
                Instruction::Rol => self.inst_rol(system, state, addr, stage), 
                Instruction::Ror => self.inst_ror(system, state, addr, stage), 
                Instruction::Rti => self.inst_rti(system, state, stage), 
                Instruction::Rts => self.inst_rts(system, state, stage), 
                Instruction::Sbc => self.inst_sbc(system, state, addr), 
                Instruction::Sec => self.inst_sec(system, state),
                Instruction::Sed => self.inst_sed(system, state),
                Instruction::Sei => self.inst_sei(system, state), 
                Instruction::Sta => self.inst_sta(system, state, addr),
                Instruction::Stx => self.inst_stx(system, state, addr), 
                Instruction::Sty => self.inst_sty(system, state, addr), 
                Instruction::Tax => self.inst_tax(system, state),
                Instruction::Tay => self.inst_tay(system, state),
                Instruction::Tsx => self.inst_tsx(system, state), 
                Instruction::Txa => self.inst_txa(system, state), 
                Instruction::Txs => self.inst_txs(system, state),
                Instruction::Tya => self.inst_tya(system, state),
                i => {
//                    println!("ILLEGAL");
                    match i {
                        Instruction::IllAhx => self.ill_inst_ahx(system, state, addr),
                        Instruction::IllAlr => self.ill_inst_alr(system, state, addr),
                        Instruction::IllAnc => self.ill_inst_anc(system, state, addr),
                        Instruction::IllArr => self.ill_inst_arr(system, state, addr),
                        Instruction::IllAxs => self.ill_inst_axs(system, state, addr),
                        Instruction::IllDcp =>
                            self.ill_inst_dcp(system, state, addr, stage),
                        Instruction::IllIsc =>
                            self.ill_inst_isc(system, state, addr, stage),
                        Instruction::IllKil => self.ill_inst_kil(system, state),
                        Instruction::IllLas => self.ill_inst_las(system, state, addr),
                        Instruction::IllLax => self.ill_inst_lax(system, state, addr),
                        Instruction::IllNop => self.ill_inst_nop(system, state, addr),
                        Instruction::IllRla =>
                            self.ill_inst_rla(system, state, addr, stage),
                        Instruction::IllRra =>
                            self.ill_inst_rra(system, state, addr, stage),
                        Instruction::IllSax => self.ill_inst_sax(system, state, addr),
                        Instruction::IllSbc => self.ill_inst_sbc(system, state, addr),
                        Instruction::IllShx => self.ill_inst_shx(system, state, addr),
                        Instruction::IllShy => self.ill_inst_shy(system, state, addr),
                        Instruction::IllSlo =>
                            self.ill_inst_slo(system, state, addr, stage),
                        Instruction::IllSre =>
                            self.ill_inst_sre(system, state, addr, stage),
                        Instruction::IllTas => self.ill_inst_tas(system, state, addr),
                        Instruction::IllXaa => self.ill_inst_xaa(system, state, addr),
                        _ => unreachable!(),
                    }
                }
            };

            match res {
                StageResult::Continue => {
                    state.cpu.stage = state.cpu.stage.increment();
                },
                StageResult::Done => {
                    state.cpu.stage = Stage::Fetch;
                },
                StageResult::Next => {
                    state.cpu.stage = Stage::Fetch;
                    self.decode(system, state);
                }
            }
        } else {
            unreachable!();
        }
    }

    fn inst_adc(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32;
        let temp = state.cpu.reg_a.wrapping_add(
            value.wrapping_add(state.cpu.flag_c));
        state.cpu.flag_v = ((!(state.cpu.reg_a ^ value) &
                             (state.cpu.reg_a ^ temp)) >> 7) & 1;
        state.cpu.flag_c  = if temp > 0xff { 1 } else { 0 };
        state.cpu.reg_a = temp & 0xff;
        state.cpu.flag_s = temp & 0xff;
        state.cpu.flag_z = temp & 0xff;
        StageResult::Done
    }

    fn inst_and(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32 & state.cpu.reg_a;
        state.cpu.reg_a = value;
        state.cpu.flag_s = value;
        state.cpu.flag_z = value;
        StageResult::Done
    }
    
    fn inst_asl(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    state.cpu.flag_c = (state.cpu.reg_a >> 7) & 1;
                    state.cpu.reg_a = (state.cpu.reg_a << 1) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    StageResult::Next
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                    StageResult::Continue
                }
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let mut value = state.cpu.decode_stack.pop_back().unwrap() as u32;
                state.cpu.flag_c = (value >> 7) & 1;
                value = (value << 1) & 0xff;
                state.cpu.flag_z = value;
                state.cpu.flag_s = value;
                self.bus.write(system, state, addr, value as u8);
                StageResult::Done
            },
            _ => unreachable!(),
        }
    }

    fn inst_branch(&self, system: &System, state: &mut SystemState, addr: u16,
    stage: u32, condition: bool) -> StageResult {
        match stage {
            0 => {
                if condition {
                    let _ = self.bus.read(system, state, addr);
                    StageResult::Continue
                } else {
                    StageResult::Next
                }
            },
            1 => {
                if addr < 0x080 {
                    if state.cpu.reg_pc & 0xff00 != 
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff00) {
                        let temp = (state.cpu.reg_pc & 0xff00) |
                            (state.cpu.reg_pc.wrapping_add(addr as u32) & 0xff);
                        let _ = self.bus.read(system, state, temp as u16);
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        StageResult::Done
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32);
                        StageResult::Next
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
                        StageResult::Done
                    } else {
                        state.cpu.reg_pc = state.cpu.reg_pc.wrapping_add(addr as u32)
                            .wrapping_sub(256);
                        StageResult::Next
                    }
                }
            },
            _ => unreachable!(),
        }
    }

    fn inst_bit(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult { 
        let value = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_s = value & 0x80;
        state.cpu.flag_v = (value >> 6) & 1;
        state.cpu.flag_z = value & state.cpu.reg_a;
        StageResult::Done
    }

    fn inst_brk(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let _ = self.bus.read(system, state, addr);
                StageResult::Continue
            },
            1 => {
                let value = state.cpu.reg_pc >> 8 & 0xff;
                self.push_stack(system, state, value as u8);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.reg_pc & 0xff;
                self.push_stack(system, state, value as u8);
                StageResult::Continue
            },
            3 => {
                let value = state.cpu.reg_p() | 0x30;
                self.push_stack(system, state, value);
                state.cpu.flag_i = 1;
                StageResult::Continue
            },
            4 => {
                let value = self.bus.read(system, state, 0xfffe);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            5 => {
                let high_value = self.bus.read(system, state, 0xffff);
                let value = state.cpu.decode_stack.pop_back().unwrap();
                state.cpu.reg_pc = value as u32 | ((high_value as u32) <<  0x8);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_clc(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_c = 0;
        StageResult::Next
    }

    fn inst_cld(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_d = 0;
        StageResult::Next
    }

    fn inst_cli(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_i = 0;
        StageResult::Next
    }

    fn inst_clv(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_v = 0;
        StageResult::Next
    }

    fn inst_cmp(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_c = if state.cpu.reg_a >= value { 1 } else { 0 };
        state.cpu.flag_z = if state.cpu.reg_a == value { 0 } else { 1 };
        state.cpu.flag_s = state.cpu.reg_a.wrapping_sub(value) & 0xff;
        StageResult::Done
    }

    fn inst_cpx(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_c = if state.cpu.reg_x >= value { 1 } else { 0 };
        state.cpu.flag_z = if state.cpu.reg_x == value { 0 } else { 1 };
        state.cpu.flag_s = state.cpu.reg_x.wrapping_sub(value) & 0xff;
        StageResult::Done
    }

    fn inst_cpy(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_c = if state.cpu.reg_y >= value { 1 } else { 0 };
        state.cpu.flag_z = if state.cpu.reg_y == value { 0 } else { 1 };
        state.cpu.flag_s = state.cpu.reg_y.wrapping_sub(value) & 0xff;
        StageResult::Done
    }

    fn inst_dec(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let value = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let value = value.wrapping_sub(1) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_dex(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_x = state.cpu.reg_x.wrapping_sub(1) & 0xff;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Next
    }
    
    fn inst_dey(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_y = state.cpu.reg_y.wrapping_sub(1) & 0xff;
        state.cpu.flag_s = state.cpu.reg_y;
        state.cpu.flag_z = state.cpu.reg_y;
        StageResult::Next
    }

    fn inst_eor(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr) as u32;
        state.cpu.reg_a ^= value;
        state.cpu.reg_a &= 0xff;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }

    fn inst_inc(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let value = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let value = value.wrapping_add(1) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            }
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_inx(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_x = state.cpu.reg_x.wrapping_add(1) & 0xff;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Next
    }

    fn inst_iny(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_y = state.cpu.reg_y.wrapping_add(1) & 0xff;
        state.cpu.flag_s = state.cpu.reg_y;
        state.cpu.flag_z = state.cpu.reg_y;
        StageResult::Next
    }

    fn inst_jmp(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        state.cpu.reg_pc = addr as u32;
        StageResult::Next
    }

    fn inst_jsr(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = state.cpu.reg_sp | 0x100;
                self.bus.read(system, state, a as u16);
                StageResult::Continue
            },
            1 => {
                let value = (state.cpu.reg_pc.wrapping_sub(1) >> 8) & 0xff;
                self.push_stack(system, state, value as u8);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.reg_pc.wrapping_sub(1) & 0xff;
                self.push_stack(system, state, value as u8);
                state.cpu.reg_pc = addr as u32;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_lda(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        state.cpu.reg_a = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }

    fn inst_ldx(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        state.cpu.reg_x = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Done
    }

    fn inst_ldy(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        state.cpu.reg_y = self.bus.read(system, state, addr) as u32;
        state.cpu.flag_s = state.cpu.reg_y;
        state.cpu.flag_z = state.cpu.reg_y;
        StageResult::Done
    }

    fn inst_lsr(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    state.cpu.flag_c = state.cpu.reg_a & 1;
                    state.cpu.reg_a >>= 1;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    StageResult::Next
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                    StageResult::Continue
                }
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                state.cpu.flag_c = (value as u32) & 1;
                let value = value >> 1;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_nop(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        StageResult::Next
    }

    fn inst_ora(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = self.bus.read(system, state, addr);
        state.cpu.reg_a = (state.cpu.reg_a | value as u32) & 0xff;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }

    fn inst_pha(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        let value = state.cpu.reg_a;
        self.push_stack(system, state, value as u8);
        StageResult::Done
    }

    fn inst_php(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        let value = state.cpu.reg_p() as u8 | 0x30;
        self.push_stack(system, state, value);
        StageResult::Done
    }

    fn inst_pla(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
                StageResult::Continue
            },
            1 => {
                state.cpu.reg_a = self.pop_stack(system, state) as u32;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_plp(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
                StageResult::Continue
            },
            1 => {
                let value = self.pop_stack(system, state) as u32;
                state.cpu.set_reg_p(value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_rol(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    let c = if state.cpu.flag_c != 0 { 1 } else { 0 };
                    state.cpu.flag_c = state.cpu.reg_a >> 7 & 1;
                    state.cpu.reg_a = (state.cpu.reg_a << 1 | c) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    StageResult::Next
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                    StageResult::Continue
                }
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let c = if state.cpu.flag_c != 0 { 1 } else { 0 };
                state.cpu.flag_c = value as u32 >> 7 & 1;
                let value = (value << 1 | c) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_ror(&self, system: &System, state: &mut SystemState, addr: u16, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                if state.cpu.op.addressing == Addressing::Accumulator {
                    let c = if state.cpu.flag_c != 0 { 0x80 } else { 0 };
                    state.cpu.flag_c = state.cpu.reg_a & 1;
                    state.cpu.reg_a = (state.cpu.reg_a >> 1 | c) & 0xff;
                    state.cpu.flag_s = state.cpu.reg_a;
                    state.cpu.flag_z = state.cpu.reg_a;
                    StageResult::Next
                } else {
                    let value = self.bus.read(system, state, addr);
                    state.cpu.decode_stack.push_back(value);
                    StageResult::Continue
                }
            },
            1 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let c = if state.cpu.flag_c != 0 { 0x80 } else { 0 };
                state.cpu.flag_c = value as u32 & 1;
                let value = (value >> 1 | c) & 0xff;
                state.cpu.flag_s = value as u32;
                state.cpu.flag_z = value as u32;
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_rti(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
                StageResult::Continue
            },
            1 => {
                let value = self.pop_stack(system, state);
                state.cpu.set_reg_p(value as u32);
                StageResult::Continue
            },
            2 => {
                let value = self.pop_stack(system, state);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            3 => {
                let high_value = (self.pop_stack(system, state) as u16) << 8;
                let value = state.cpu.decode_stack.pop_back().unwrap() as u16;
                state.cpu.reg_pc = (high_value | value) as u32;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_rts(&self, system: &System, state: &mut SystemState, stage: u32)
    -> StageResult {
        match stage {
            0 => {
                let a = state.cpu.reg_sp | 0x100;
                let _ = self.bus.read(system, state, a as u16);
                StageResult::Continue
            },
            1 => {
                let value = self.pop_stack(system, state);
                state.cpu.decode_stack.push_back(value);
                StageResult::Continue
            },
            2 => {
                let high_value = (self.pop_stack(system, state) as u16) << 8;
                let value = state.cpu.decode_stack.pop_back().unwrap() as u16;
                state.cpu.reg_pc = (high_value | value).wrapping_add(1) as u32;
                StageResult::Continue
            },
            3 => {
                let a = state.cpu.reg_pc;
                let _ = self.bus.read(system, state, a as u16);
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn inst_sbc(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
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
        StageResult::Done
    }

    fn inst_sec(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_c = 1;
        StageResult::Next 
    }

    fn inst_sed(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_d = 1;
        StageResult::Next 
    }

    fn inst_sei(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.flag_i = 1;
        StageResult::Next 
    }

    fn inst_sta(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = state.cpu.reg_a;
        self.bus.write(system, state, addr, value as u8); 
        StageResult::Done
    }

    fn inst_stx(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = state.cpu.reg_x;
        self.bus.write(system, state, addr, value as u8); 
        StageResult::Done
    }

    fn inst_sty(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let value = state.cpu.reg_y;
        self.bus.write(system, state, addr, value as u8); 
        StageResult::Done
    }

    fn inst_tax(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_x = state.cpu.reg_a;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Next
    }

    fn inst_tay(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_y = state.cpu.reg_a;
        state.cpu.flag_s = state.cpu.reg_y;
        state.cpu.flag_z = state.cpu.reg_y;
        StageResult::Next
    }

    fn inst_tsx(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_x = state.cpu.reg_sp;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Next
    }

    fn inst_txa(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_a = state.cpu.reg_x;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Next
    }

    fn inst_txs(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_sp = state.cpu.reg_x;
        StageResult::Next
    }

    fn inst_tya(&self, system: &System, state: &mut SystemState)
    -> StageResult {
        state.cpu.reg_a = state.cpu.reg_y;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Next
    }

    fn ill_inst_ahx(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        self.bus.read(system, state, addr);
        StageResult::Done
    }

    fn ill_inst_alr(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let val = self.bus.read(system, state, addr);
        state.cpu.reg_a &= val as u32;
        state.cpu.flag_c = state.cpu.reg_a & 1;
        state.cpu.reg_a >>= 1;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }

    fn ill_inst_anc(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let val = self.bus.read(system, state, addr);
        state.cpu.reg_a &= val as u32;
        state.cpu.flag_c = (state.cpu.reg_a >> 7) & 1;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }

    fn ill_inst_arr(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        let val = self.bus.read(system, state, addr);
        state.cpu.reg_a &= val as u32;
        if state.cpu.flag_c != 0 {
            state.cpu.flag_c = state.cpu.reg_a & 1;
            state.cpu.reg_a = ((state.cpu.reg_a >> 1) | 0x80) & 0xff;
            state.cpu.flag_s = state.cpu.reg_a;
            state.cpu.flag_z = state.cpu.reg_a;
        } else {
            state.cpu.flag_c = state.cpu.reg_a & 1;
            state.cpu.reg_a = (state.cpu.reg_a >> 1) & 0xff;
            state.cpu.flag_s = state.cpu.reg_a;
            state.cpu.flag_z = state.cpu.reg_a;
        }
        match ((state.cpu.reg_a & 0x40), (state.cpu.reg_a & 0x20)) {
            (0,0) =>{
                state.cpu.flag_c = 0;
                state.cpu.flag_v = 0;
            },
            (_,0) => {
                state.cpu.flag_c = 1;
                state.cpu.flag_v = 1;
            },
            (0,_) => {
                state.cpu.flag_c = 0;
                state.cpu.flag_v = 1;
            },
            (_,_) => {
                state.cpu.flag_c = 1;
                state.cpu.flag_v = 0;
            }
        }
        StageResult::Done
    }

    fn ill_inst_axs(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
        state.cpu.reg_x &= state.cpu.reg_a;
        let val = self.bus.read(system, state, addr);
        let temp = state.cpu.reg_x.wrapping_sub(val as u32);
        state.cpu.flag_c = if temp > state.cpu.reg_x { 0 } else { 1 };
        state.cpu.reg_x = temp & 0xff;
        state.cpu.flag_s = state.cpu.reg_x;
        state.cpu.flag_z = state.cpu.reg_x;
        StageResult::Done
    }
    
    fn ill_inst_dcp(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                let val = val.wrapping_sub(1);
                state.cpu.flag_s = val as u32;
                state.cpu.flag_z = val as u32;
                self.bus.write(system, state, addr, val);
                state.cpu.flag_c = if state.cpu.reg_a >= val as u32 { 1 } else { 0 };
                state.cpu.flag_z = if state.cpu.reg_a == val as u32 { 0 } else { 1 };
                state.cpu.flag_s = state.cpu.reg_a.wrapping_sub(val as u32) & 0xff;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }
    
    fn ill_inst_isc(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                let val = val.wrapping_add(1);
                self.bus.write(system, state, addr, val);
                let val = val as i32;
                let temp_a = state.cpu.reg_a as i32;
                let temp = temp_a.wrapping_sub(
                    val.wrapping_sub(state.cpu.flag_c as i32 - 1));
                state.cpu.flag_v = (((temp_a ^ val) &
                                     (temp_a ^ temp)) >> 7) as u32 & 1;
                state.cpu.flag_c  = if temp < 0 { 0 } else { 1 };
                state.cpu.reg_a = (temp as u32) & 0xff;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn ill_inst_kil(&self, system: &System, state: &mut SystemState) -> StageResult {
        println!("KIL encountered");
        StageResult::Next
    }

    fn ill_inst_las(&self, system: &System, state: &mut SystemState, addr: u16)
            -> StageResult {
        let _ = self.bus.read(system, state, addr);
        StageResult::Done
    }
    
    fn ill_inst_lax(&self, system: &System, state: &mut SystemState, addr: u16) -> StageResult {
        let val = self.bus.read(system, state, addr);
        state.cpu.reg_a = val as u32;
        state.cpu.reg_x = state.cpu.reg_a;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }
    
    fn ill_inst_nop(&self, system: &System, state: &mut SystemState, addr: u16) -> StageResult {
        match state.cpu.op.addressing {
            Addressing::Immediate | Addressing::ZeroPage  |
            Addressing::Absolute  | Addressing::ZeroPageX |
            Addressing::AbsoluteX(_) => {
                let _ = self.bus.read(system, state, addr);
                StageResult::Done
            },
            _ => {
                StageResult::Next
            }
        }
    }
    
    fn ill_inst_rla(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                let c = if state.cpu.flag_c != 0 { 1 } else { 0 };
                state.cpu.flag_c = value as u32 >> 7 & 1;
                let value = (value << 1 | c) & 0xff;
                self.bus.write(system, state, addr, value);
                state.cpu.reg_a &= value as u32;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn ill_inst_rra(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let value = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, value);
                let value = value as u32;
                let c = if state.cpu.flag_c != 0 { 0x80 } else { 0 };
                state.cpu.flag_c = value as u32 & 1;
                let value = (value >> 1 | c) & 0xff;
                self.bus.write(system, state, addr, value as u8);
                let temp = state.cpu.reg_a.wrapping_add(
                    value.wrapping_add(state.cpu.flag_c));
                state.cpu.flag_v = ((!(state.cpu.reg_a ^ value) &
                                     (state.cpu.reg_a ^ temp)) >> 7) & 1;
                state.cpu.flag_c  = if temp > 0xff { 1 } else { 0 };
                state.cpu.reg_a = temp & 0xff;
                state.cpu.flag_s = temp & 0xff;
                state.cpu.flag_z = temp & 0xff;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }
    
    fn ill_inst_sax(&self, system: &System, state: &mut SystemState, addr: u16) -> StageResult {
        let val = (state.cpu.reg_a & state.cpu.reg_x) & 0xff;
        self.bus.write(system, state, addr, val as u8);
        StageResult::Done
    }

    fn ill_inst_sbc(&self, system: &System, state: &mut SystemState, addr: u16)
    -> StageResult {
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
        StageResult::Done
    }
    
    fn ill_inst_shx(&self, system: &System, state: &mut SystemState,
                    addr: u16) -> StageResult {
        let temp_addr = addr as u32;
        let value = (state.cpu.reg_x & ((temp_addr >> 8).wrapping_add(1))) & 0xff;
        let temp = temp_addr.wrapping_sub(state.cpu.reg_y) & 0xff;
        if state.cpu.reg_y.wrapping_add(temp) <= 0xff {
            self.bus.write(system, state, addr, value as u8);
        } else {
            let value = self.bus.peek(system, state, addr);
            self.bus.write(system, state, addr, value);
        }
        StageResult::Done
    }

    fn ill_inst_shy(&self, system: &System, state: &mut SystemState,
                    addr: u16) -> StageResult {
        let temp_addr = addr as u32;
        let value = (state.cpu.reg_y & ((temp_addr >> 8).wrapping_add(1))) & 0xff;
        let temp = temp_addr.wrapping_sub(state.cpu.reg_x) & 0xff;
        if state.cpu.reg_x.wrapping_add(temp) <= 0xff {
            self.bus.write(system, state, addr, value as u8);
        } else {
            let value = self.bus.peek(system, state, addr);
            self.bus.write(system, state, addr, value);
        }
        StageResult::Done
    }

    fn ill_inst_slo(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let mut value = state.cpu.decode_stack.pop_back().unwrap() as u32;
                state.cpu.flag_c = (value >> 7) & 1;
                value = (value << 1) & 0xff;
                self.bus.write(system, state, addr, value as u8);
                state.cpu.reg_a |= value;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }
    
    fn ill_inst_sre(&self, system: &System, state: &mut SystemState, addr: u16,
                    stage: u32) -> StageResult {
        match stage {
            0 => {
                let val = self.bus.read(system, state, addr);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            1 => {
                let val = state.cpu.decode_stack.pop_back().unwrap();
                self.bus.write(system, state, addr, val);
                state.cpu.decode_stack.push_back(val);
                StageResult::Continue
            },
            2 => {
                let mut value = state.cpu.decode_stack.pop_back().unwrap() as u32;
                state.cpu.flag_c = value & 1;
                value >>= 1;
                self.bus.write(system, state, addr, value as u8);
                state.cpu.reg_a ^= value;
                state.cpu.reg_a &= 0xff;
                state.cpu.flag_s = state.cpu.reg_a;
                state.cpu.flag_z = state.cpu.reg_a;
                StageResult::Done
            },
            _ => unreachable!()
        }
    }

    fn ill_inst_tas(&self, system: &System, state: &mut SystemState,
                    addr: u16) -> StageResult {
        state.cpu.reg_sp = state.cpu.reg_x & state.cpu.reg_a;
        let val = state.cpu.reg_sp & ((addr as u32) >> 8);
        self.bus.write(system, state, addr, val as u8);
        StageResult::Done
    }

    fn ill_inst_xaa(&self, system: &System, state: &mut SystemState,
                    addr: u16) -> StageResult {
        let val = self.bus.read(system, state, addr) as u32;
        state.cpu.reg_a = state.cpu.reg_x & val;
        state.cpu.flag_s = state.cpu.reg_a;
        state.cpu.flag_z = state.cpu.reg_a;
        StageResult::Done
    }
    
    fn will_wrap(addr: u16, add: u16) -> bool {
        addr & 0xff00 != addr.wrapping_add(add) & 0xff00
    }

    fn wrapping_add(addr: u16, add: u16) -> u16 {
        (addr & 0xff00) | (addr.wrapping_add(add) & 0xff)
    }
}

