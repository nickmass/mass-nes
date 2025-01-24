#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

pub mod dma;
mod interrupts;
pub mod ops;
mod registers;

use dma::Dma;
use interrupts::Interrupts;
use ops::*;
use registers::CpuRegs;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Copy, Clone)]
pub struct CpuPinIn {
    pub data: u8,
    pub irq: bool,
    pub reset: bool,
    pub power: bool,
    pub nmi: bool,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum TickResult {
    Fetch(u16),
    Read(u16),
    Write(u16, u8),
    Idle(u16),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum AddressResult {
    Address(u16),
    TickAddress(TickResult, u16),
    Next(TickResult, Addressing),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum ExecResult {
    Done,
    Next(TickResult, Instruction),
    Tick(TickResult),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Stage {
    Fetch,
    Decode,
    Address(Addressing, Instruction),
    Execute(u16, Instruction),
}

#[allow(dead_code)]
#[cfg(feature = "debugger")]
#[derive(Debug, Copy, Clone, Default)]
pub struct CpuDebugState {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u8,
    pub reg_p: u8,
    pub instruction_addr: Option<u16>,
    pub cycle: u64,
}

#[cfg(not(feature = "debugger"))]
#[derive(Debug, Copy, Clone, Default)]
pub struct CpuDebugState;

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Cpu {
    current_tick: u64,
    pin_in: CpuPinIn,
    regs: CpuRegs,
    stage: Stage,
    pub dma: Dma,
    interrupts: Interrupts,
    halt: bool,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            current_tick: 0,
            pin_in: Default::default(),
            regs: CpuRegs::new(),
            stage: Stage::Fetch,
            dma: Dma::new(),
            interrupts: Interrupts::new(),
            halt: false,
        }
    }

    pub fn power_up_pc(&mut self, pc: Option<u16>) {
        if let Some(pc) = pc {
            self.interrupts.with_power_up_pc(pc);
        }
    }

    #[cfg(feature = "debugger")]
    pub fn debug_state(&self) -> CpuDebugState {
        CpuDebugState {
            reg_a: self.regs.reg_a,
            reg_x: self.regs.reg_x,
            reg_y: self.regs.reg_y,
            reg_sp: self.regs.reg_sp,
            reg_p: self.regs.reg_p(),
            reg_pc: self.regs.reg_pc,
            instruction_addr: None,
            cycle: self.current_tick,
        }
    }
    #[cfg(not(feature = "debugger"))]
    pub fn debug_state(&self) -> CpuDebugState {
        CpuDebugState
    }

    pub fn tick(&mut self, pin_in: CpuPinIn) -> TickResult {
        self.pin_in = pin_in;

        if pin_in.power {
            self.halt = false;
        } else if self.halt {
            return TickResult::Idle(0xffff);
        }

        self.current_tick += 1;

        let tick = if let Some(result) = self.dma.tick(pin_in) {
            result
        } else {
            let tick = self.step();
            self.dma.try_halt(tick).unwrap_or(tick)
        };

        self.interrupts.tick(&pin_in);

        tick
    }

    fn step(&mut self) -> TickResult {
        match self.stage {
            Stage::Fetch => self.fetch(),
            Stage::Decode => self.decode(),
            Stage::Address(addressing, instruction) => self.addressing(addressing, instruction),
            Stage::Execute(address, instruction) => self.execute(address, instruction),
        }
    }

    fn fetch(&mut self) -> TickResult {
        if let Some(tick) = self.interrupts.interrupt(&self.pin_in, &mut self.regs) {
            tick
        } else {
            self.stage = Stage::Decode;
            self.regs.fetch_pc()
        }
    }

    fn decode(&mut self) -> TickResult {
        let op = OPS[self.pin_in.data as usize];
        self.addressing(op.addressing, op.instruction)
    }

    fn addressing(&mut self, addressing: Addressing, instruction: Instruction) -> TickResult {
        use Addressing::*;
        let address_res = match addressing {
            None => self.addr_none(),
            Accumulator => self.addr_accumulator(),
            Immediate => self.addr_immediate(),
            ZeroPage(step) => self.addr_zero_page(step),
            ZeroPageOffset(reg, step) => self.addr_zero_page_offset(reg, step),
            Absolute(step) => self.addr_absolute(step),
            AbsoluteOffset(reg, dummy, step) => self.addr_absolute_offset(reg, dummy, step),
            IndirectAbsolute(step) => self.addr_indirect_absolute(step),
            Relative(step) => self.addr_relative(step),
            IndirectX(step) => self.addr_indirect_x(step),
            IndirectY(dummy, step) => self.addr_indirect_y(dummy, step),
        };

        match address_res {
            AddressResult::Next(tick, next) => {
                self.stage = Stage::Address(next, instruction);
                tick
            }
            AddressResult::TickAddress(tick, address) => {
                self.stage = Stage::Execute(address, instruction);
                tick
            }
            AddressResult::Address(addr) => self.execute(addr, instruction),
        }
    }

    fn addr_none(&mut self) -> AddressResult {
        AddressResult::TickAddress(TickResult::Read(self.regs.reg_pc), 0x0000)
    }

    fn addr_accumulator(&mut self) -> AddressResult {
        AddressResult::TickAddress(TickResult::Read(self.regs.reg_pc), self.regs.reg_a as u16)
    }

    fn addr_immediate(&mut self) -> AddressResult {
        let addr = self.regs.reg_pc;
        self.regs.reg_pc = self.regs.reg_pc.wrapping_add(1);
        AddressResult::Address(addr)
    }

    fn addr_zero_page(&mut self, step: ZeroPage) -> AddressResult {
        use AddressResult::*;
        use ZeroPage::*;
        match step {
            Read => Next(self.regs.read_pc(), Addressing::ZeroPage(Decode)),
            Decode => Address(self.pin_in.data as u16),
        }
    }

    fn addr_zero_page_offset(&mut self, reg: Reg, step: ZeroPageOffset) -> AddressResult {
        use AddressResult::*;
        use ZeroPageOffset::*;
        match step {
            ReadImmediate => {
                let next = Addressing::ZeroPageOffset(reg, ApplyOffset);
                Next(self.regs.read_pc(), next)
            }
            ApplyOffset => {
                let reg = match reg {
                    Reg::X => self.regs.reg_x,
                    Reg::Y => self.regs.reg_y,
                };
                let addr = self.pin_in.data.wrapping_add(reg);
                TickAddress(TickResult::Read(self.pin_in.data as u16), addr as u16)
            }
        }
    }

    fn addr_absolute(&mut self, step: Absolute) -> AddressResult {
        use Absolute::*;
        use AddressResult::*;
        match step {
            ReadLow => {
                let next = Addressing::Absolute(ReadHigh);
                Next(self.regs.read_pc(), next)
            }
            ReadHigh => {
                let low_addr = self.pin_in.data as u16;
                let next = Addressing::Absolute(Decode(low_addr));
                Next(self.regs.read_pc(), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = low_addr | high_addr;
                Address(addr)
            }
        }
    }

    fn addr_absolute_offset(
        &mut self,
        reg: Reg,
        dummy: DummyRead,
        step: AbsoluteOffset,
    ) -> AddressResult {
        use AbsoluteOffset::*;
        use AddressResult::*;
        match step {
            ReadLow => {
                let next = Addressing::AbsoluteOffset(reg, dummy, ReadHigh);
                Next(self.regs.read_pc(), next)
            }
            ReadHigh => {
                let next = Addressing::AbsoluteOffset(reg, dummy, Decode(self.pin_in.data as u16));
                Next(self.regs.read_pc(), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = high_addr | low_addr;
                let reg = match reg {
                    Reg::X => self.regs.reg_x,
                    Reg::Y => self.regs.reg_y,
                };
                let reg = (reg & 0xff) as u16;
                let offset_addr = addr.wrapping_add(reg);
                let will_wrap = will_wrap(addr, reg);
                match (will_wrap, dummy) {
                    (true, DummyRead::OnCarry) | (_, DummyRead::Always) => {
                        let dummy_addr = wrapping_add(addr, reg);
                        TickAddress(TickResult::Read(dummy_addr), offset_addr)
                    }
                    _ => Address(offset_addr),
                }
            }
        }
    }

    fn addr_indirect_absolute(&mut self, step: IndirectAbsolute) -> AddressResult {
        use AddressResult::*;
        use IndirectAbsolute::*;
        match step {
            ReadLow => {
                let next = Addressing::IndirectAbsolute(ReadHigh);
                Next(self.regs.read_pc(), next)
            }
            ReadHigh => {
                let next = Addressing::IndirectAbsolute(ReadIndirectLow(self.pin_in.data as u16));
                Next(self.regs.read_pc(), next)
            }
            ReadIndirectLow(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = low_addr | high_addr;
                let next = Addressing::IndirectAbsolute(ReadIndirectHigh(addr));
                Next(TickResult::Read(addr), next)
            }
            ReadIndirectHigh(addr) => {
                let addr = wrapping_add(addr, 1);
                let next = Addressing::IndirectAbsolute(Decode(self.pin_in.data as u16));
                Next(TickResult::Read(addr), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = low_addr | high_addr;
                Address(addr)
            }
        }
    }

    fn addr_relative(&mut self, step: Relative) -> AddressResult {
        use AddressResult::*;
        use Relative::*;
        match step {
            ReadRegPc => Next(self.regs.read_pc(), Addressing::Relative(Decode)),
            Decode => Address(self.pin_in.data as u16),
        }
    }

    fn addr_indirect_x(&mut self, step: IndirectX) -> AddressResult {
        use AddressResult::*;
        use IndirectX::*;
        match step {
            ReadBase => {
                let next = Addressing::IndirectX(ReadDummy);
                Next(self.regs.read_pc(), next)
            }
            ReadDummy => {
                let addr = self.pin_in.data.wrapping_add(self.regs.reg_x) as u16;
                let next = Addressing::IndirectX(ReadIndirectLow(addr));
                Next(TickResult::Read(self.pin_in.data as u16), next)
            }
            ReadIndirectLow(offset_addr) => {
                let next = Addressing::IndirectX(ReadIndirectHigh(offset_addr));
                Next(TickResult::Read(offset_addr), next)
            }
            ReadIndirectHigh(offset_addr) => {
                let next = Addressing::IndirectX(Decode(self.pin_in.data as u16));
                let high_offset_addr = wrapping_add(offset_addr, 1);
                Next(TickResult::Read(high_offset_addr), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = low_addr | high_addr;
                Address(addr)
            }
        }
    }

    fn addr_indirect_y(&mut self, dummy: DummyRead, step: IndirectY) -> AddressResult {
        use AddressResult::*;
        use IndirectY::*;
        match step {
            ReadBase => {
                let next = Addressing::IndirectY(dummy, ReadZeroPageLow);
                Next(self.regs.read_pc(), next)
            }
            ReadZeroPageLow => {
                let zp_low_addr = self.pin_in.data as u16;
                let next = Addressing::IndirectY(dummy, ReadZeroPageHigh(zp_low_addr));
                Next(TickResult::Read(zp_low_addr), next)
            }
            ReadZeroPageHigh(zp_low_addr) => {
                let zp_high_addr = wrapping_add(zp_low_addr, 1);
                let low_addr = self.pin_in.data as u16;
                let next = Addressing::IndirectY(dummy, Decode(low_addr));
                Next(TickResult::Read(zp_high_addr), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = low_addr | high_addr;
                let reg_y = self.regs.reg_y as u16;
                let offset_addr = addr.wrapping_add(reg_y);
                let will_wrap = will_wrap(addr, reg_y);
                match (will_wrap, dummy) {
                    (true, DummyRead::OnCarry) | (_, DummyRead::Always) => {
                        let dummy_addr = wrapping_add(addr, reg_y);
                        TickAddress(TickResult::Read(dummy_addr), offset_addr)
                    }
                    _ => Address(offset_addr),
                }
            }
        }
    }

    fn execute(&mut self, address: u16, instruction: Instruction) -> TickResult {
        use Instruction::*;
        let exec_result = match instruction {
            Adc(step) => self.inst_adc(address, step),
            And(step) => self.inst_and(address, step),
            Asl(step) => self.inst_asl(address, step),
            Asla => self.inst_asla(),
            Bcc(step) => self.inst_branch(address, step, !self.regs.flag_c),
            Bcs(step) => self.inst_branch(address, step, self.regs.flag_c),
            Beq(step) => self.inst_branch(address, step, self.regs.flag_z),
            Bit(step) => self.inst_bit(address, step),
            Bmi(step) => self.inst_branch(address, step, self.regs.flag_s),
            Bne(step) => self.inst_branch(address, step, !self.regs.flag_z),
            Bpl(step) => self.inst_branch(address, step, !self.regs.flag_s),
            Brk(step) => self.inst_brk(address, step),
            Bvc(step) => self.inst_branch(address, step, !self.regs.flag_v),
            Bvs(step) => self.inst_branch(address, step, self.regs.flag_v),
            Clc => self.inst_clc(),
            Cld => self.inst_cld(),
            Cli => self.inst_cli(),
            Clv => self.inst_clv(),
            Cmp(step) => self.inst_cmp(address, step),
            Cpx(step) => self.inst_cpx(address, step),
            Cpy(step) => self.inst_cpy(address, step),
            Dec(step) => self.inst_dec(address, step),
            Dex => self.inst_dex(),
            Dey => self.inst_dey(),
            Eor(step) => self.inst_eor(address, step),
            Inc(step) => self.inst_inc(address, step),
            Inx => self.inst_inx(),
            Iny => self.inst_iny(),
            Jmp => self.inst_jmp(address),
            Jsr(step) => self.inst_jsr(address, step),
            Lda(step) => self.inst_lda(address, step),
            Ldx(step) => self.inst_ldx(address, step),
            Ldy(step) => self.inst_ldy(address, step),
            Lsr(step) => self.inst_lsr(address, step),
            Lsra => self.inst_lsra(),
            Nop => self.inst_nop(),
            Ora(step) => self.inst_ora(address, step),
            Pha => self.inst_pha(),
            Php => self.inst_php(),
            Pla(step) => self.inst_pla(step),
            Plp(step) => self.inst_plp(step),
            Rol(step) => self.inst_rol(address, step),
            Rola => self.inst_rola(),
            Ror(step) => self.inst_ror(address, step),
            Rora => self.inst_rora(),
            Rti(step) => self.inst_rti(step),
            Rts(step) => self.inst_rts(step),
            Sbc(step) => self.inst_sbc(address, step),
            Sec => self.inst_sec(),
            Sed => self.inst_sed(),
            Sei => self.inst_sei(),
            Sta => self.inst_sta(address),
            Stx => self.inst_stx(address),
            Sty => self.inst_sty(address),
            Tax => self.inst_tax(),
            Tay => self.inst_tay(),
            Tsx => self.inst_tsx(),
            Txa => self.inst_txa(),
            Txs => self.inst_txs(),
            Tya => self.inst_tya(),

            IllAhx => self.ill_inst_ahx(address),
            IllAlr(step) => self.ill_inst_alr(address, step),
            IllAnc(step) => self.ill_inst_anc(address, step),
            IllArr(step) => self.ill_inst_arr(address, step),
            IllAxs(step) => self.ill_inst_axs(address, step),
            IllDcp(step) => self.ill_inst_dcp(address, step),
            IllIsc(step) => self.ill_inst_isc(address, step),
            IllKil => self.ill_inst_kil(),
            IllLas(step) => self.ill_inst_las(address, step),
            IllLax(step) => self.ill_inst_lax(address, step),
            IllNop => self.ill_inst_nop(),
            IllNopAddr => self.ill_inst_nop_addr(address),
            IllRla(step) => self.ill_inst_rla(address, step),
            IllRra(step) => self.ill_inst_rra(address, step),
            IllSax => self.ill_inst_sax(address),
            IllSbc(step) => self.ill_inst_sbc(address, step),
            IllShx => self.ill_inst_shx(address),
            IllShy => self.ill_inst_shy(address),
            IllSlo(step) => self.ill_inst_slo(address, step),
            IllSre(step) => self.ill_inst_sre(address, step),
            IllTas => self.ill_inst_tas(address),
            IllXaa(step) => self.ill_inst_xaa(address, step),
        };

        match exec_result {
            ExecResult::Next(tick, next) => {
                self.stage = Stage::Execute(address, next);
                tick
            }
            ExecResult::Tick(tick) => {
                // setup no-op for final phase of last tick
                self.stage = Stage::Execute(0x0000, Nop);
                tick
            }
            ExecResult::Done => {
                self.stage = Stage::Fetch;
                self.fetch()
            }
        }
    }

    fn inst_adc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        match step {
            ReadExec::Read => {
                ExecResult::Next(TickResult::Read(addr), Instruction::Adc(ReadExec::Exec))
            }
            ReadExec::Exec => {
                self.interrupts.poll(&self.regs);
                let data = self.pin_in.data as u32;
                let reg_a = self.regs.reg_a as u32;
                let tmp_reg_a = reg_a;
                let reg_a = reg_a.wrapping_add(data.wrapping_add(self.regs.flag_c as u32));
                self.regs.flag_v = ((!(tmp_reg_a ^ data) & (tmp_reg_a ^ reg_a)) & 0x80) != 0;
                self.regs.flag_c = reg_a > 0xff;
                self.regs.reg_a = (reg_a & 0xff) as u8;
                self.regs.set_flags_zs(self.regs.reg_a);

                ExecResult::Done
            }
        }
    }

    fn inst_and(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        match step {
            ReadExec::Read => {
                ExecResult::Next(TickResult::Read(addr), Instruction::And(ReadExec::Exec))
            }
            ReadExec::Exec => {
                self.interrupts.poll(&self.regs);
                let data = self.pin_in.data;
                self.regs.reg_a &= data;
                self.regs.set_flags_zs(self.regs.reg_a);

                ExecResult::Done
            }
        }
    }

    fn inst_asl(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Asl(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(TickResult::Write(addr, data), Instruction::Asl(Exec(data)))
            }
            Exec(data) => {
                let value = self.asl(data);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_asla(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.asl(self.regs.reg_a);
        ExecResult::Done
    }

    fn asl(&mut self, mut value: u8) -> u8 {
        self.regs.flag_c = value & 0x80 != 0;
        value = (value << 1) & 0xff;
        self.regs.set_flags_zs(value);

        value
    }

    fn inst_branch(&mut self, addr: u16, step: Branch, condition: bool) -> ExecResult {
        use self::Branch::*;
        use ExecResult::*;
        match step {
            Check => {
                self.interrupts.poll(&self.regs);
                if condition {
                    // TODO: Messy setting it to BCC
                    Next(TickResult::Read(addr), Instruction::Bcc(Branch))
                } else {
                    Done
                }
            }
            Branch => {
                let high_pc = self.regs.reg_pc & 0xff00;
                if addr < 0x080 {
                    let offset_pc = self.regs.reg_pc.wrapping_add(addr);
                    self.regs.reg_pc = offset_pc;
                    if high_pc != offset_pc & 0xff00 {
                        let dummy_pc = (high_pc | (offset_pc & 0xff)) as u16;
                        Tick(TickResult::Read(dummy_pc))
                    } else {
                        Done
                    }
                } else {
                    let offset_pc = self.regs.reg_pc.wrapping_add(addr).wrapping_sub(256);
                    self.regs.reg_pc = offset_pc;
                    if high_pc != (offset_pc & 0xff00) {
                        let dummy_pc = high_pc | (offset_pc & 0xff);
                        Tick(TickResult::Read(dummy_pc))
                    } else {
                        Done
                    }
                }
            }
        }
    }

    fn inst_bit(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Bit(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let data = self.pin_in.data;
                self.regs.flag_v = ((data >> 6) & 1) != 0;
                self.regs.flag_z = (data & self.regs.reg_a) == 0;
                self.regs.flag_s = data & 0x80 != 0;

                Done
            }
        }
    }

    fn inst_brk(&mut self, addr: u16, step: Break) -> ExecResult {
        use Break::*;
        use ExecResult::*;
        match step {
            ReadDummy => Next(TickResult::Read(addr), Instruction::Brk(WriteRegPcHigh)),
            WriteRegPcHigh => {
                let pc_high = ((self.regs.reg_pc >> 8) & 0xff) as u8;
                Next(
                    self.regs.push_stack(pc_high),
                    Instruction::Brk(WriteRegPcLow),
                )
            }
            WriteRegPcLow => {
                let pc_low = (self.regs.reg_pc & 0xff) as u8;
                Next(self.regs.push_stack(pc_low), Instruction::Brk(WriteRegP))
            }
            WriteRegP => {
                let reg_p = self.regs.reg_p() | 0x30;
                self.regs.flag_i = true;
                Next(self.regs.push_stack(reg_p), Instruction::Brk(ReadHighJump))
            }
            ReadHighJump => {
                self.interrupts.poll(&self.regs);
                let vector = if let Some(trigger) = self.interrupts.triggered() {
                    trigger.vector()
                } else {
                    0xfffe
                };
                Next(
                    TickResult::Read(vector),
                    Instruction::Brk(ReadLowJump(vector)),
                )
            }
            ReadLowJump(addr) => {
                let low_value = self.pin_in.data as u16;
                Next(
                    TickResult::Read(addr + 1),
                    Instruction::Brk(UpdateRegPc(low_value)),
                )
            }
            UpdateRegPc(low_value) => {
                let high_value = (self.pin_in.data as u16) << 8;
                self.regs.reg_pc = low_value | high_value;
                Done
            }
        }
    }

    fn inst_clc(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_c = false;
        ExecResult::Done
    }

    fn inst_cld(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_d = false;
        ExecResult::Done
    }

    fn inst_cli(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_i = false;
        ExecResult::Done
    }

    fn inst_clv(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_v = false;
        ExecResult::Done
    }

    fn inst_cmp(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Cmp(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data;
                self.regs.flag_c = self.regs.reg_a >= value;
                let value = self.regs.reg_a.wrapping_sub(value);
                self.regs.set_flags_zs(value);
                Done
            }
        }
    }

    fn inst_cpx(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Cpx(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data;
                self.regs.flag_c = self.regs.reg_x >= value;
                let value = self.regs.reg_x.wrapping_sub(value);
                self.regs.set_flags_zs(value);
                Done
            }
        }
    }

    fn inst_cpy(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Cpy(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data;
                self.regs.flag_c = self.regs.reg_y >= value;
                let value = self.regs.reg_y.wrapping_sub(value);
                self.regs.set_flags_zs(value);
                Done
            }
        }
    }

    fn inst_dec(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Dec(Dummy)),
            Dummy => {
                let value = self.pin_in.data;
                Next(
                    TickResult::Write(addr, value),
                    Instruction::Dec(Exec(value)),
                )
            }
            Exec(value) => {
                let value = value.wrapping_sub(1);
                self.regs.set_flags_zs(value);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_dex(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_x = self.regs.reg_x.wrapping_sub(1);
        self.regs.set_flags_zs(self.regs.reg_x);
        ExecResult::Done
    }

    fn inst_dey(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_y = self.regs.reg_y.wrapping_sub(1);
        self.regs.set_flags_zs(self.regs.reg_y);
        ExecResult::Done
    }

    fn inst_eor(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Eor(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data;
                self.regs.reg_a ^= value;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn inst_inc(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Inc(Dummy)),
            Dummy => {
                let value = self.pin_in.data;
                Next(
                    TickResult::Write(addr, value),
                    Instruction::Inc(Exec(value)),
                )
            }
            Exec(value) => {
                let value = value.wrapping_add(1);
                self.regs.set_flags_zs(value);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_inx(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_x = self.regs.reg_x.wrapping_add(1);
        self.regs.set_flags_zs(self.regs.reg_x);
        ExecResult::Done
    }

    fn inst_iny(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_y = self.regs.reg_y.wrapping_add(1);
        self.regs.set_flags_zs(self.regs.reg_y);
        ExecResult::Done
    }

    fn inst_jmp(&mut self, addr: u16) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_pc = addr;
        ExecResult::Done
    }

    fn inst_jsr(&mut self, addr: u16, step: Jsr) -> ExecResult {
        use ExecResult::*;
        use Jsr::*;
        match step {
            ReadDummy => {
                let dummy_addr = self.regs.reg_sp as u16 | 0x100;
                Next(
                    TickResult::Read(dummy_addr),
                    Instruction::Jsr(WriteRegPcHigh),
                )
            }
            WriteRegPcHigh => {
                let value = (self.regs.reg_pc.wrapping_sub(1) >> 8) & 0xff;
                Next(
                    self.regs.push_stack(value as u8),
                    Instruction::Jsr(WriteRegPcLow),
                )
            }
            WriteRegPcLow => {
                let value = self.regs.reg_pc.wrapping_sub(1) & 0xff;
                self.regs.reg_pc = addr;
                Tick(self.regs.push_stack(value as u8))
            }
        }
    }

    fn inst_lda(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Lda(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a = self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn inst_ldx(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Ldx(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_x = self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_x);
                Done
            }
        }
    }

    fn inst_ldy(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Ldy(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_y = self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_y);
                Done
            }
        }
    }

    fn inst_lsr(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Lsr(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(TickResult::Write(addr, data), Instruction::Lsr(Exec(data)))
            }
            Exec(data) => {
                let value = self.lsr(data);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_lsra(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.lsr(self.regs.reg_a);

        ExecResult::Done
    }

    fn lsr(&mut self, value: u8) -> u8 {
        self.regs.flag_c = (value & 1) != 0;
        let value = value >> 1;
        self.regs.set_flags_zs(value);

        value
    }

    fn inst_nop(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        ExecResult::Done
    }

    fn inst_ora(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Ora(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a = self.regs.reg_a | self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn inst_pha(&mut self) -> ExecResult {
        ExecResult::Tick(self.regs.push_stack(self.regs.reg_a))
    }

    fn inst_php(&mut self) -> ExecResult {
        let value = self.regs.reg_p() | 0x30;
        ExecResult::Tick(self.regs.push_stack(value))
    }

    fn inst_pla(&mut self, step: DummyReadExec) -> ExecResult {
        use DummyReadExec::*;
        use ExecResult::*;
        match step {
            Dummy => {
                let dummy_addr = self.regs.reg_sp as u16 | 0x100;
                Next(TickResult::Read(dummy_addr), Instruction::Pla(Read))
            }
            Read => Next(self.regs.pop_stack(), Instruction::Pla(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a = self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn inst_plp(&mut self, step: DummyReadExec) -> ExecResult {
        use DummyReadExec::*;
        use ExecResult::*;
        match step {
            Dummy => {
                let dummy_addr = self.regs.reg_sp as u16 | 0x100;
                Next(TickResult::Read(dummy_addr), Instruction::Plp(Read))
            }
            Read => Next(self.regs.pop_stack(), Instruction::Plp(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data;
                self.regs.set_reg_p(value);
                Done
            }
        }
    }

    fn inst_rol(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Rol(Dummy)),
            Dummy => {
                let value = self.pin_in.data;
                Next(
                    TickResult::Write(addr, value),
                    Instruction::Rol(Exec(value)),
                )
            }
            Exec(data) => {
                let value = self.rol(data);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_rola(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.rol(self.regs.reg_a);

        ExecResult::Done
    }

    fn rol(&mut self, value: u8) -> u8 {
        let c = if self.regs.flag_c { 1 } else { 0 };
        self.regs.flag_c = value & 0x80 != 0;
        let value = value << 1 | c;
        self.regs.set_flags_zs(value);

        value
    }

    fn inst_ror(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Ror(Dummy)),
            Dummy => {
                let value = self.pin_in.data;
                Next(
                    TickResult::Write(addr, value),
                    Instruction::Ror(Exec(value)),
                )
            }
            Exec(data) => {
                let value = self.ror(data);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_rora(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.ror(self.regs.reg_a);

        ExecResult::Done
    }

    fn ror(&mut self, value: u8) -> u8 {
        let c = if self.regs.flag_c { 0x80 } else { 0 };
        self.regs.flag_c = value & 1 != 0;
        let value = value >> 1 | c;
        self.regs.set_flags_zs(value);

        value
    }

    fn inst_rti(&mut self, step: Rti) -> ExecResult {
        use ExecResult::*;
        use Rti::*;
        match step {
            Dummy => {
                let dummy_addr = self.regs.reg_sp as u16 | 0x100;
                Next(TickResult::Read(dummy_addr), Instruction::Rti(ReadRegP))
            }
            ReadRegP => Next(self.regs.pop_stack(), Instruction::Rti(ReadRegPcLow)),
            ReadRegPcLow => {
                let reg_p = self.pin_in.data;
                self.regs.set_reg_p(reg_p);
                Next(self.regs.pop_stack(), Instruction::Rti(ReadRegPcHigh))
            }
            ReadRegPcHigh => {
                let low_value = self.pin_in.data;
                Next(
                    self.regs.pop_stack(),
                    Instruction::Rti(Exec(low_value as u16)),
                )
            }
            Exec(low_addr) => {
                self.interrupts.poll(&self.regs);
                let high_addr = (self.pin_in.data as u16) << 8;
                self.regs.reg_pc = high_addr | low_addr;
                Done
            }
        }
    }

    fn inst_rts(&mut self, step: Rts) -> ExecResult {
        use ExecResult::*;
        use Rts::*;
        match step {
            Dummy => {
                let dummy_addr = self.regs.reg_sp as u16 | 0x100;
                Next(TickResult::Read(dummy_addr), Instruction::Rts(ReadRegPcLow))
            }
            ReadRegPcLow => Next(self.regs.pop_stack(), Instruction::Rts(ReadRegPcHigh)),
            ReadRegPcHigh => {
                let low_value = self.pin_in.data as u16;
                Next(self.regs.pop_stack(), Instruction::Rts(Exec(low_value)))
            }
            Exec(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                self.regs.reg_pc = (high_addr | low_addr).wrapping_add(1);
                Tick(TickResult::Read(self.regs.reg_pc))
            }
        }
    }

    fn inst_sbc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Sbc(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data as i32;
                let temp_a = self.regs.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.regs.flag_c as i32 - 1));
                self.regs.flag_v = ((temp_a ^ value) & (temp_a ^ temp)) & 0x80 != 0;
                self.regs.flag_c = temp >= 0;
                self.regs.reg_a = temp as u8;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn inst_sec(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_c = true;
        ExecResult::Done
    }

    fn inst_sed(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_d = true;
        ExecResult::Done
    }

    fn inst_sei(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.flag_i = true;
        ExecResult::Done
    }

    fn inst_sta(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.regs.reg_a))
    }

    fn inst_stx(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.regs.reg_x))
    }

    fn inst_sty(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.regs.reg_y))
    }

    fn inst_tax(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_x = self.regs.reg_a;
        self.regs.set_flags_zs(self.regs.reg_x);
        ExecResult::Done
    }

    fn inst_tay(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_y = self.regs.reg_a;
        self.regs.set_flags_zs(self.regs.reg_y);
        ExecResult::Done
    }

    fn inst_tsx(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_x = self.regs.reg_sp;
        self.regs.set_flags_zs(self.regs.reg_x);
        ExecResult::Done
    }

    fn inst_txa(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.regs.reg_x;
        self.regs.set_flags_zs(self.regs.reg_a);
        ExecResult::Done
    }

    fn inst_txs(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_sp = self.regs.reg_x;
        ExecResult::Done
    }

    fn inst_tya(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.regs.reg_a = self.regs.reg_y;
        self.regs.set_flags_zs(self.regs.reg_a);
        ExecResult::Done
    }

    fn ill_inst_ahx(&mut self, addr: u16) -> ExecResult {
        let base_addr = addr.wrapping_sub(self.regs.reg_y as u16);
        let hi = ((base_addr >> 8) as u8).wrapping_add(1);
        let value = self.regs.reg_a & self.regs.reg_x & hi;

        let wrapped = addr & 0xff00 != base_addr & 0xff00;
        let target = if wrapped {
            let hi_a = (self.regs.reg_a as u16) << 8 | 0xff;
            let hi_x = (self.regs.reg_x as u16) << 8 | 0xff;
            addr & hi_a & hi_x
        } else {
            addr
        };

        ExecResult::Tick(TickResult::Write(target, value))
    }

    fn ill_inst_alr(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllAlr(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a &= self.pin_in.data;
                self.regs.flag_c = self.regs.reg_a & 1 != 0;
                self.regs.reg_a >>= 1;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn ill_inst_anc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllAnc(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a &= self.pin_in.data;
                self.regs.flag_c = self.regs.reg_a & 0x80 != 0;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn ill_inst_arr(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllArr(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a &= self.pin_in.data;

                let c = if self.regs.flag_c { 0x80 } else { 0x00 };
                self.regs.flag_c = self.regs.reg_a & 1 != 0;
                self.regs.reg_a = (self.regs.reg_a >> 1) | c;
                self.regs.set_flags_zs(self.regs.reg_a);

                match ((self.regs.reg_a & 0x40), (self.regs.reg_a & 0x20)) {
                    (0, 0) => {
                        self.regs.flag_c = false;
                        self.regs.flag_v = false;
                    }
                    (_, 0) => {
                        self.regs.flag_c = true;
                        self.regs.flag_v = true;
                    }
                    (0, _) => {
                        self.regs.flag_c = false;
                        self.regs.flag_v = true;
                    }
                    (_, _) => {
                        self.regs.flag_c = true;
                        self.regs.flag_v = false;
                    }
                }
                Done
            }
        }
    }

    fn ill_inst_axs(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllAxs(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_x &= self.regs.reg_a;
                let temp = self.regs.reg_x.wrapping_sub(self.pin_in.data);
                self.regs.flag_c = temp <= self.regs.reg_x;
                self.regs.reg_x = temp;
                self.regs.set_flags_zs(self.regs.reg_x);
                Done
            }
        }
    }

    fn ill_inst_dcp(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllDcp(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllDcp(Exec(data)),
                )
            }
            Exec(data) => {
                let value = data.wrapping_sub(1);
                self.regs.flag_c = self.regs.reg_a >= value;
                let tmp = self.regs.reg_a.wrapping_sub(value);
                self.regs.set_flags_zs(tmp);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn ill_inst_isc(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllIsc(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllIsc(Exec(data)),
                )
            }
            Exec(data) => {
                let value = data.wrapping_add(1) as i32;
                let temp_a = self.regs.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.regs.flag_c as i32 - 1));
                self.regs.flag_v = ((temp_a ^ value) & (temp_a ^ temp)) & 0x80 != 0;
                self.regs.flag_c = temp >= 0;
                self.regs.reg_a = temp as u8;
                self.regs.set_flags_zs(self.regs.reg_a);
                Tick(TickResult::Write(addr, value as u8))
            }
        }
    }

    fn ill_inst_kil(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        self.halt = true;
        tracing::error!("KIL encountered");
        ExecResult::Done
    }

    fn ill_inst_las(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllLas(Exec)),
            Exec => {
                self.regs.reg_sp &= self.pin_in.data;
                self.regs.reg_a = self.regs.reg_sp;
                self.regs.reg_x = self.regs.reg_sp;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn ill_inst_lax(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllLax(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a = self.pin_in.data;
                self.regs.reg_x = self.regs.reg_a;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn ill_inst_nop(&mut self) -> ExecResult {
        self.interrupts.poll(&self.regs);
        ExecResult::Done
    }

    fn ill_inst_nop_addr(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Read(addr))
    }

    fn ill_inst_rla(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllRla(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllRla(Exec(data)),
                )
            }
            Exec(data) => {
                let c = self.regs.flag_c as u8;
                self.regs.flag_c = data & 0x80 != 0;
                let value = (data << 1) | c;
                self.regs.reg_a &= value;
                self.regs.set_flags_zs(self.regs.reg_a);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn ill_inst_rra(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllRra(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllRra(Exec(data)),
                )
            }
            Exec(data) => {
                let data = data as u32;
                let c = if self.regs.flag_c { 0x80 } else { 0 };
                self.regs.flag_c = data & 1 != 0;
                let data = (data >> 1 | c) & 0xff;
                let a = self.regs.reg_a as u32;
                let value = a.wrapping_add(data.wrapping_add(self.regs.flag_c as u32));
                self.regs.flag_v = (!(a ^ data) & (a ^ value)) & 0x80 != 0;
                self.regs.flag_c = value > 0xff;
                self.regs.reg_a = value as u8;
                self.regs.set_flags_zs(self.regs.reg_a);

                Tick(TickResult::Write(addr, data as u8))
            }
        }
    }

    fn ill_inst_sax(&mut self, addr: u16) -> ExecResult {
        let value = self.regs.reg_a & self.regs.reg_x;
        ExecResult::Tick(TickResult::Write(addr, value as u8))
    }

    fn ill_inst_sbc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllSbc(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                let value = self.pin_in.data as i32;
                let temp_a = self.regs.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.regs.flag_c as i32 - 1));
                self.regs.flag_v = ((temp_a ^ value) & (temp_a ^ temp)) & 0x80 != 0;
                self.regs.flag_c = temp >= 0;
                self.regs.reg_a = temp as u8;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }

    fn ill_inst_shx(&mut self, addr: u16) -> ExecResult {
        let base_addr = addr.wrapping_sub(self.regs.reg_y as u16);
        let hi = ((base_addr >> 8) as u8).wrapping_add(1);
        let value = self.regs.reg_x & hi;

        let wrapped = addr & 0xff00 != base_addr & 0xff00;
        let target = if wrapped {
            let hi_x = (self.regs.reg_x as u16) << 8 | 0xff;
            addr & hi_x
        } else {
            addr
        };

        ExecResult::Tick(TickResult::Write(target, value))
    }

    fn ill_inst_shy(&mut self, addr: u16) -> ExecResult {
        let base_addr = addr.wrapping_sub(self.regs.reg_x as u16);
        let hi = ((base_addr >> 8) as u8).wrapping_add(1);
        let value = self.regs.reg_y & hi;

        let wrapped = addr & 0xff00 != base_addr & 0xff00;
        let target = if wrapped {
            let hi_y = (self.regs.reg_y as u16) << 8 | 0xff;
            addr & hi_y
        } else {
            addr
        };

        ExecResult::Tick(TickResult::Write(target, value))
    }

    fn ill_inst_slo(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllSlo(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllSlo(Exec(data)),
                )
            }
            Exec(data) => {
                self.regs.flag_c = data & 0x80 != 0;
                let value = data << 1;
                self.regs.reg_a |= value;
                self.regs.set_flags_zs(self.regs.reg_a);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn ill_inst_sre(&mut self, addr: u16, step: ReadDummyExec) -> ExecResult {
        use ExecResult::*;
        use ReadDummyExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllSre(Dummy)),
            Dummy => {
                let data = self.pin_in.data;
                Next(
                    TickResult::Write(addr, data),
                    Instruction::IllSre(Exec(data)),
                )
            }
            Exec(data) => {
                self.regs.flag_c = data & 1 != 0;
                let value = data >> 1;
                self.regs.reg_a ^= value;
                self.regs.set_flags_zs(self.regs.reg_a);
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn ill_inst_tas(&mut self, addr: u16) -> ExecResult {
        let base_addr = addr.wrapping_sub(self.regs.reg_y as u16);
        let hi = ((base_addr >> 8) as u8).wrapping_add(1);
        self.regs.reg_sp = self.regs.reg_a & self.regs.reg_x;
        let value = self.regs.reg_a & self.regs.reg_x & hi;

        let wrapped = addr & 0xff00 != base_addr & 0xff00;
        let target = if wrapped {
            let hi_a = (self.regs.reg_a as u16) << 8 | 0xff;
            let hi_x = (self.regs.reg_x as u16) << 8 | 0xff;
            addr & hi_a & hi_x
        } else {
            addr
        };

        ExecResult::Tick(TickResult::Write(target, value))
    }

    fn ill_inst_xaa(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllXaa(Exec)),
            Exec => {
                self.interrupts.poll(&self.regs);
                self.regs.reg_a = self.regs.reg_x & self.pin_in.data;
                self.regs.set_flags_zs(self.regs.reg_a);
                Done
            }
        }
    }
}

fn will_wrap(addr: u16, add: u16) -> bool {
    addr & 0xff00 != addr.wrapping_add(add) & 0xff00
}

fn wrapping_add(addr: u16, add: u16) -> u16 {
    (addr & 0xff00) | (addr.wrapping_add(add) & 0xff)
}
