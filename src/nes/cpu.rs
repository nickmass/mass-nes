use crate::ops::*;
use std::cell::Cell;

#[derive(Default, Debug, Copy, Clone)]
pub struct CpuPinIn {
    pub data: u8,
    pub irq: bool,
    pub nmi: bool,
    pub reset: bool,
    pub power: bool,
    pub dmc_req: Option<u16>,
}

#[derive(Debug, Copy, Clone)]
enum PendingDmcRead {
    Pending(u16, u32),
    Reading,
}

#[derive(Debug, Copy, Clone)]
enum OamDma {
    Read(u16, u16),
    Write(u16, u16),
}

#[derive(Debug, Copy, Clone)]
enum Irq {
    ReadPcOne(u16),
    ReadPcTwo(u16),
    WriteRegPcHigh(u16),
    WriteRegPcLow(u16),
    WriteRegP(u16),
    ReadHighJump(u16),
    ReadLowJump(u16),
    UpdateRegPc,
}

#[derive(Debug, Copy, Clone)]
enum Power {
    ReadRegPcLow,
    ReadRegPcHigh,
    UpdateRegPc(u16),
}

#[derive(Debug, Copy, Clone)]
pub enum TickResult {
    Read(u16),
    Write(u16, u8),
    Idle,
}

#[derive(Copy, Clone)]
enum AddressResult {
    Address(u16),
    TickAddress(TickResult, u16),
    Next(TickResult, Addressing),
}

#[derive(Copy, Clone)]
enum ExecResult {
    Done,
    Next(TickResult, Instruction),
    Tick(TickResult),
}

#[derive(Debug, Copy, Clone)]
enum Stage {
    Fetch,
    Decode,
    Address(Addressing, Instruction),
    Execute(u16, Instruction),
    OamDma(OamDma),
    Reset(Power),
    Power(Power),
    Irq(Irq),
}

#[derive(Debug, Copy, Clone)]
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

pub struct Cpu {
    current_tick: u64,
    ops: &'static [Op; 0x100],
    power_up_pc: Option<u16>,
    pin_in: CpuPinIn,
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
    stage: Stage,
    last_tick: TickResult,
    instruction_addr: Option<u16>,
    pub dmc_read: Option<u8>,
    dmc_hold: u8,
    pending_dmc: Option<PendingDmcRead>,
    pending_nmi: Cell<Option<u32>>,
    pending_oam_dma: Cell<Option<u8>>,
    pending_power: bool,
    pending_reset: bool,
    irq_delay: u32,
    irq_set_delay: u32,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            current_tick: 0,
            ops: Op::load(),
            power_up_pc: None,
            pin_in: Default::default(),
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_pc: 0,
            reg_sp: 0,
            flag_c: 0,
            flag_z: 0,
            flag_i: 0,
            flag_d: 0,
            flag_v: 0,
            flag_s: 0,
            instruction_addr: None,
            last_tick: TickResult::Read(0),
            dmc_read: None,
            dmc_hold: 0,
            stage: Stage::Fetch,
            pending_dmc: None,
            pending_nmi: Cell::new(None),
            pending_oam_dma: Cell::new(None),
            pending_power: false,
            pending_reset: false,
            irq_delay: 0,
            irq_set_delay: 0,
        }
    }

    pub fn power_up_pc(&mut self, pc: Option<u16>) {
        self.power_up_pc = pc;
    }

    fn power(&mut self, step: Power) -> TickResult {
        use Power::*;
        use TickResult::*;
        match step {
            ReadRegPcLow => {
                self.stage = Stage::Power(ReadRegPcHigh);
                Read(0xfffc)
            }
            ReadRegPcHigh => {
                self.stage = Stage::Power(UpdateRegPc(self.pin_in.data as u16));
                Read(0xfffc + 1)
            }
            UpdateRegPc(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                self.reg_pc = (low_addr | high_addr) as u32;
                self.set_reg_p(0x34);
                self.reg_sp = 0xfd;
                if let Some(addr) = self.power_up_pc {
                    self.reg_pc = addr as u32;
                }
                self.fetch()
            }
        }
    }

    fn reset(&mut self, step: Power) -> TickResult {
        use Power::*;
        use TickResult::*;
        match step {
            ReadRegPcLow => {
                self.stage = Stage::Reset(ReadRegPcHigh);
                Read(0xfffc)
            }
            ReadRegPcHigh => {
                self.stage = Stage::Reset(UpdateRegPc(self.pin_in.data as u16));
                Read(0xfffc + 1)
            }
            UpdateRegPc(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                self.reg_pc = (low_addr | high_addr) as u32;
                self.reg_sp = self.reg_sp.wrapping_sub(3);
                self.flag_i = 1;
                self.fetch()
            }
        }
    }

    fn reg_p(&self) -> u8 {
        let mut val = 0;
        if self.flag_c != 0 {
            val |= 0x01;
        }
        if self.flag_z == 0 {
            val |= 0x02;
        }
        if self.flag_i != 0 {
            val |= 0x04;
        }
        if self.flag_d != 0 {
            val |= 0x08;
        }
        if self.flag_v != 0 {
            val |= 0x40;
        }
        if self.flag_s & 0x80 != 0 {
            val |= 0x80;
        }

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

    pub fn oam_dma_req(&self, addr: u8) {
        self.pending_oam_dma.set(Some(addr));
    }

    pub fn nmi_req(&self, delay: u32) {
        self.pending_nmi.set(Some(delay));
    }

    fn dmc_req(&mut self) {
        if let Some(addr) = self.pin_in.dmc_req {
            self.pending_dmc = Some(PendingDmcRead::Pending(addr, 4));
        }
    }

    pub fn nmi_cancel(&self) {
        self.pending_nmi.set(None);
    }

    pub fn tick(&mut self, pin_in: CpuPinIn) -> TickResult {
        self.pin_in = pin_in;
        self.dmc_req();
        self.instruction_addr = None;
        self.dmc_read = None;
        self.current_tick += 1;

        if self.pin_in.power {
            self.pin_in.power = false;
            self.pending_power = true;
        }
        if self.pin_in.reset {
            self.pin_in.reset = false;
            self.pending_reset = true;
        }

        match self.pending_dmc {
            Some(PendingDmcRead::Pending(addr, 0)) => {
                self.pending_dmc = Some(PendingDmcRead::Reading);
                return TickResult::Read(addr);
            }
            Some(PendingDmcRead::Pending(addr, count)) => {
                self.pending_dmc = Some(PendingDmcRead::Pending(addr, count - 1));
                match self.last_tick {
                    TickResult::Read(_) => {
                        self.dmc_hold = self.pin_in.data;
                        return TickResult::Idle;
                    }
                    _ => (),
                }
            }
            Some(PendingDmcRead::Reading) => {
                self.dmc_read = Some(self.pin_in.data);
                self.pending_dmc = None;
                self.pin_in.data = self.dmc_hold
            }
            None => (),
        }

        self.last_tick = match self.stage {
            Stage::Fetch => self.fetch(),
            Stage::Decode => self.decode(),
            Stage::Address(addressing, instruction) => self.addressing(addressing, instruction),
            Stage::Execute(address, instruction) => self.execute(address, instruction),
            Stage::OamDma(oam) => self.oam_dma(oam),
            Stage::Irq(irq) => self.irq_nmi(irq),
            Stage::Power(step) => self.power(step),
            Stage::Reset(step) => self.reset(step),
        };

        self.last_tick
    }

    pub fn debug_state(&self) -> CpuDebugState {
        CpuDebugState {
            reg_a: self.reg_a as u8,
            reg_x: self.reg_x as u8,
            reg_y: self.reg_y as u8,
            reg_sp: self.reg_sp as u8,
            reg_p: self.reg_p(),
            reg_pc: self.reg_pc as u16,
            instruction_addr: self.instruction_addr,
            cycle: self.current_tick,
        }
    }

    fn read_pc(&mut self) -> TickResult {
        let pc = self.reg_pc as u16;
        self.reg_pc = pc.wrapping_add(1) as u32;
        TickResult::Read(pc)
    }

    fn pop_stack(&mut self) -> TickResult {
        self.reg_sp = self.reg_sp.wrapping_add(1) & 0xff;
        let addr = self.reg_sp as u16 | 0x100;
        TickResult::Read(addr)
    }

    fn push_stack(&mut self, value: u8) -> TickResult {
        let addr = self.reg_sp as u16 | 0x100;
        self.reg_sp = self.reg_sp.wrapping_sub(1) & 0xff;
        TickResult::Write(addr, value)
    }

    fn oam_dma(&mut self, oam: OamDma) -> TickResult {
        match oam {
            OamDma::Read(high_addr, low_addr) => {
                self.stage = Stage::OamDma(OamDma::Write(high_addr, low_addr));
                TickResult::Read(high_addr | low_addr)
            }
            OamDma::Write(high_addr, low_addr) => {
                if low_addr == 255 {
                    self.stage = Stage::Fetch;
                } else {
                    self.stage = Stage::OamDma(OamDma::Read(high_addr, low_addr + 1));
                }
                TickResult::Write(0x2004, self.pin_in.data)
            }
        }
    }

    fn irq_nmi(&mut self, irq: Irq) -> TickResult {
        use self::Irq::*;
        match irq {
            ReadPcOne(addr) => {
                self.stage = Stage::Irq(Irq::ReadPcTwo(addr));
                TickResult::Read(self.reg_pc as u16)
            }
            ReadPcTwo(addr) => {
                self.stage = Stage::Irq(Irq::WriteRegPcHigh(addr));
                TickResult::Read(self.reg_pc as u16)
            }
            WriteRegPcHigh(addr) => {
                self.stage = Stage::Irq(Irq::WriteRegPcLow(addr));
                let val = (self.reg_pc >> 8) & 0xff;
                self.push_stack(val as u8)
            }
            WriteRegPcLow(addr) => {
                self.stage = Stage::Irq(Irq::WriteRegP(addr));
                let val = self.reg_pc & 0xff;
                self.push_stack(val as u8)
            }
            WriteRegP(addr) => {
                if self.pending_nmi.get().is_some() {
                    self.pending_nmi.set(None);
                    self.stage = Stage::Irq(Irq::ReadHighJump(0xfffa));
                } else {
                    self.stage = Stage::Irq(Irq::ReadHighJump(addr));
                }
                let val = self.reg_p() | 0x20;
                self.push_stack(val)
            }
            ReadHighJump(addr) => {
                self.stage = Stage::Irq(Irq::ReadLowJump(addr));
                TickResult::Read(addr)
            }
            ReadLowJump(addr) => {
                self.stage = Stage::Irq(Irq::UpdateRegPc);
                self.reg_pc &= 0xff00;
                self.reg_pc |= self.pin_in.data as u32;
                self.flag_i = 1;
                TickResult::Read(addr + 1)
            }
            UpdateRegPc => {
                self.reg_pc &= 0x00ff;
                self.reg_pc |= ((self.pin_in.data as u16) << 8) as u32;
                self.fetch()
            }
        }
    }

    fn interrupt(&mut self) -> Stage {
        let mut stage = Stage::Decode;
        match self.pending_nmi.get() {
            Some(0) => {
                self.pending_nmi.set(None);
                stage = Stage::Irq(Irq::ReadPcOne(0xfffa));
            }
            Some(count) => {
                self.pending_nmi.set(Some(count - 1));
            }
            None => (),
        }

        if self.pin_in.irq && (self.flag_i == 0 || self.irq_set_delay != 0) && self.irq_delay == 0 {
            if self.irq_set_delay != 0 {
                self.irq_set_delay -= 1;
            }
            stage = Stage::Irq(Irq::ReadPcOne(0xfffe))
        }
        if self.irq_set_delay != 0 {
            self.irq_set_delay -= 1;
        }
        if self.irq_delay != 0 {
            self.irq_delay -= 1;
        }
        if let Some(high_addr) = self.pending_oam_dma.get() {
            self.pending_oam_dma.set(None);
            stage = Stage::OamDma(OamDma::Read((high_addr as u16) << 8, 0))
        }
        if self.pending_power {
            self.pending_power = false;
            stage = Stage::Power(Power::ReadRegPcLow);
        }

        if self.pending_reset {
            self.pending_reset = false;
            stage = Stage::Reset(Power::ReadRegPcLow);
        }

        stage
    }

    fn fetch(&mut self) -> TickResult {
        self.stage = self.interrupt();

        match self.stage {
            Stage::Fetch => self.read_pc(),
            Stage::Decode => {
                self.instruction_addr = Some(self.reg_pc as u16);
                self.read_pc()
            }
            Stage::Address(addressing, instruction) => self.addressing(addressing, instruction),
            Stage::Execute(address, instruction) => self.execute(address, instruction),
            Stage::OamDma(oam) => self.oam_dma(oam),
            Stage::Irq(irq) => self.irq_nmi(irq),
            Stage::Power(step) => self.power(step),
            Stage::Reset(step) => self.reset(step),
        }
    }

    fn decode(&mut self) -> TickResult {
        let op = self.ops[self.pin_in.data as usize];
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
        let dummy_addr = (self.reg_pc as u16).wrapping_add(1);
        AddressResult::TickAddress(TickResult::Read(dummy_addr), 0x0000)
    }

    fn addr_accumulator(&mut self) -> AddressResult {
        let dummy_addr = (self.reg_pc as u16).wrapping_add(1);
        AddressResult::TickAddress(TickResult::Read(dummy_addr), self.reg_a as u16)
    }

    fn addr_immediate(&mut self) -> AddressResult {
        let addr = self.reg_pc as u16;
        self.reg_pc = self.reg_pc.wrapping_add(1);
        AddressResult::Address(addr)
    }

    fn addr_zero_page(&mut self, step: ZeroPage) -> AddressResult {
        use AddressResult::*;
        use ZeroPage::*;
        match step {
            Read => Next(self.read_pc(), Addressing::ZeroPage(Decode)),
            Decode => Address(self.pin_in.data as u16),
        }
    }

    fn addr_zero_page_offset(&mut self, reg: Reg, step: ZeroPageOffset) -> AddressResult {
        use AddressResult::*;
        use ZeroPageOffset::*;
        match step {
            ReadImmediate => {
                let next = Addressing::ZeroPageOffset(reg, ApplyOffset);
                Next(self.read_pc(), next)
            }
            ApplyOffset => {
                let reg = match reg {
                    Reg::X => self.reg_x,
                    Reg::Y => self.reg_y,
                };
                let addr = self.pin_in.data.wrapping_add(reg as u8);
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
                Next(self.read_pc(), next)
            }
            ReadHigh => {
                let low_addr = self.pin_in.data as u16;
                let next = Addressing::Absolute(Decode(low_addr));
                Next(self.read_pc(), next)
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
                Next(self.read_pc(), next)
            }
            ReadHigh => {
                let next = Addressing::AbsoluteOffset(reg, dummy, Decode(self.pin_in.data as u16));
                Next(self.read_pc(), next)
            }
            Decode(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                let addr = high_addr | low_addr;
                let reg = match reg {
                    Reg::X => self.reg_x,
                    Reg::Y => self.reg_y,
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
                Next(self.read_pc(), next)
            }
            ReadHigh => {
                let next = Addressing::IndirectAbsolute(ReadIndirectLow(self.pin_in.data as u16));
                Next(self.read_pc(), next)
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
            ReadRegPc => Next(self.read_pc(), Addressing::Relative(Decode)),
            Decode => Address(self.pin_in.data as u16),
        }
    }

    fn addr_indirect_x(&mut self, step: IndirectX) -> AddressResult {
        use AddressResult::*;
        use IndirectX::*;
        match step {
            ReadBase => {
                let next = Addressing::IndirectX(ReadDummy);
                Next(self.read_pc(), next)
            }
            ReadDummy => {
                let addr = (self.pin_in.data.wrapping_add(self.reg_x as u8) & 0xff) as u16;
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
                Next(self.read_pc(), next)
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
                let reg_y = (self.reg_y & 0xff) as u16;
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
            Bcc(step) => {
                let cond = self.flag_c == 0;
                self.inst_branch(address, step, cond)
            }
            Bcs(step) => {
                let cond = self.flag_c != 0;
                self.inst_branch(address, step, cond)
            }
            Beq(step) => {
                let cond = self.flag_z == 0;
                self.inst_branch(address, step, cond)
            }
            Bit(step) => self.inst_bit(address, step),
            Bmi(step) => {
                let cond = self.flag_s & 0x80 != 0;
                self.inst_branch(address, step, cond)
            }
            Bne(step) => {
                let cond = self.flag_z != 0;
                self.inst_branch(address, step, cond)
            }
            Bpl(step) => {
                let cond = self.flag_s & 0x80 == 0;
                self.inst_branch(address, step, cond)
            }
            Brk(step) => self.inst_brk(address, step),
            Bvc(step) => {
                let cond = self.flag_v == 0;
                self.inst_branch(address, step, cond)
            }
            Bvs(step) => {
                let cond = self.flag_v != 0;
                self.inst_branch(address, step, cond)
            }
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
            IllLas => self.ill_inst_las(address),
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
                self.stage = Stage::Fetch;
                tick
            }
            ExecResult::Done => self.fetch(),
        }
    }

    fn inst_adc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        match step {
            ReadExec::Read => {
                ExecResult::Next(TickResult::Read(addr), Instruction::Adc(ReadExec::Exec))
            }
            ReadExec::Exec => {
                let data = self.pin_in.data as u32;
                let reg_a = self.reg_a.wrapping_add(data.wrapping_add(self.flag_c));
                self.flag_v = ((!(self.reg_a ^ data) & (self.reg_a ^ reg_a)) >> 7) & 1;
                self.flag_c = if reg_a > 0xff { 1 } else { 0 };
                self.reg_a = reg_a & 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;

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
                let data = self.pin_in.data as u32;
                self.reg_a &= data;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;

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
                let value = self.asl(data as u32) as u8;
                Tick(TickResult::Write(addr, value))
            }
        }
    }

    fn inst_asla(&mut self) -> ExecResult {
        self.reg_a = self.asl(self.reg_a);
        ExecResult::Done
    }

    fn asl(&mut self, mut value: u32) -> u32 {
        self.flag_c = (value >> 7) & 1;
        value = (value << 1) & 0xff;
        self.flag_z = value;
        self.flag_s = value;

        value
    }

    fn inst_branch(&mut self, addr: u16, step: Branch, condition: bool) -> ExecResult {
        use self::Branch::*;
        use ExecResult::*;
        match step {
            Check => {
                if condition {
                    // TODO: Messy setting it to BCC
                    Next(TickResult::Read(addr), Instruction::Bcc(Branch))
                } else {
                    Done
                }
            }
            Branch => {
                let high_pc = self.reg_pc & 0xff00;
                if addr < 0x080 {
                    let offset_pc = self.reg_pc.wrapping_add(addr as u32);
                    self.reg_pc = offset_pc;
                    if high_pc != offset_pc & 0xff00 {
                        let dummy_pc = (high_pc | (offset_pc & 0xff)) as u16;
                        Tick(TickResult::Read(dummy_pc))
                    } else {
                        Done
                    }
                } else {
                    let offset_pc = self.reg_pc.wrapping_add(addr as u32).wrapping_sub(256);
                    self.reg_pc = offset_pc;
                    if high_pc != (offset_pc & 0xff00) {
                        let dummy_pc = (high_pc | (offset_pc & 0xff)) as u16;
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
                let data = self.pin_in.data as u32;
                self.flag_s = data & 0x80;
                self.flag_v = (data >> 6) & 1;
                self.flag_z = data & self.reg_a;

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
                let pc_high = ((self.reg_pc >> 8) & 0xff) as u8;
                Next(self.push_stack(pc_high), Instruction::Brk(WriteRegPcLow))
            }
            WriteRegPcLow => {
                let pc_low = (self.reg_pc & 0xff) as u8;
                Next(self.push_stack(pc_low), Instruction::Brk(WriteRegP))
            }
            WriteRegP => {
                let reg_p = self.reg_p() | 0x30;
                self.flag_i = 1;
                let jump = if self.pending_nmi.get().is_some() {
                    self.pending_nmi.set(None);
                    ReadHighJump(0xfffa)
                } else {
                    ReadHighJump(0xfffe)
                };
                Next(self.push_stack(reg_p), Instruction::Brk(jump))
            }
            ReadHighJump(addr) => Next(TickResult::Read(addr), Instruction::Brk(ReadLowJump(addr))),
            ReadLowJump(addr) => {
                let low_value = self.pin_in.data as u16;
                Next(
                    TickResult::Read(addr + 1),
                    Instruction::Brk(UpdateRegPc(low_value)),
                )
            }
            UpdateRegPc(low_value) => {
                let high_value = (self.pin_in.data as u16) << 8;
                self.reg_pc = (low_value | high_value) as u32;
                Done
            }
        }
    }

    fn inst_clc(&mut self) -> ExecResult {
        self.flag_c = 0;
        ExecResult::Done
    }

    fn inst_cld(&mut self) -> ExecResult {
        self.flag_d = 0;
        ExecResult::Done
    }

    fn inst_cli(&mut self) -> ExecResult {
        if self.flag_i == 1 {
            self.irq_delay = 1;
        }
        self.flag_i = 0;
        ExecResult::Done
    }

    fn inst_clv(&mut self) -> ExecResult {
        self.flag_v = 0;
        ExecResult::Done
    }

    fn inst_cmp(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Cmp(Exec)),
            Exec => {
                let value = self.pin_in.data as u32;
                self.flag_c = if self.reg_a >= value { 1 } else { 0 };
                self.flag_z = if self.reg_a == value { 0 } else { 1 };
                self.flag_s = self.reg_a.wrapping_sub(value) & 0xff;
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
                let value = self.pin_in.data as u32;
                self.flag_c = if self.reg_x >= value { 1 } else { 0 };
                self.flag_z = if self.reg_x == value { 0 } else { 1 };
                self.flag_s = self.reg_x.wrapping_sub(value) & 0xff;
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
                let value = self.pin_in.data as u32;
                self.flag_c = if self.reg_y >= value { 1 } else { 0 };
                self.flag_z = if self.reg_y == value { 0 } else { 1 };
                self.flag_s = self.reg_y.wrapping_sub(value) & 0xff;
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
                let value = value.wrapping_sub(1) as u32;
                self.flag_s = value;
                self.flag_z = value;
                Tick(TickResult::Write(addr, value as u8))
            }
        }
    }

    fn inst_dex(&mut self) -> ExecResult {
        self.reg_x = self.reg_x.wrapping_sub(1) & 0xff;
        self.flag_s = self.reg_x;
        self.flag_z = self.reg_x;
        ExecResult::Done
    }

    fn inst_dey(&mut self) -> ExecResult {
        self.reg_y = self.reg_y.wrapping_sub(1) & 0xff;
        self.flag_s = self.reg_y;
        self.flag_z = self.reg_y;
        ExecResult::Done
    }

    fn inst_eor(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Eor(Exec)),
            Exec => {
                let value = self.pin_in.data as u32;
                self.reg_a ^= value;
                self.reg_a &= 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
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
                let value = value.wrapping_add(1) as u32;
                self.flag_s = value;
                self.flag_z = value;
                Tick(TickResult::Write(addr, value as u8))
            }
        }
    }

    fn inst_inx(&mut self) -> ExecResult {
        self.reg_x = self.reg_x.wrapping_add(1) & 0xff;
        self.flag_s = self.reg_x;
        self.flag_z = self.reg_x;
        ExecResult::Done
    }

    fn inst_iny(&mut self) -> ExecResult {
        self.reg_y = self.reg_y.wrapping_add(1) & 0xff;
        self.flag_s = self.reg_y;
        self.flag_z = self.reg_y;
        ExecResult::Done
    }

    fn inst_jmp(&mut self, addr: u16) -> ExecResult {
        self.reg_pc = addr as u32;
        ExecResult::Done
    }

    fn inst_jsr(&mut self, addr: u16, step: Jsr) -> ExecResult {
        use ExecResult::*;
        use Jsr::*;
        match step {
            ReadDummy => {
                let dummy_addr = self.reg_sp | 0x100;
                Next(
                    TickResult::Read(dummy_addr as u16),
                    Instruction::Jsr(WriteRegPcHigh),
                )
            }
            WriteRegPcHigh => {
                let value = (self.reg_pc.wrapping_sub(1) >> 8) & 0xff;
                Next(
                    self.push_stack(value as u8),
                    Instruction::Jsr(WriteRegPcLow),
                )
            }
            WriteRegPcLow => {
                let value = self.reg_pc.wrapping_sub(1) & 0xff;
                self.reg_pc = addr as u32;
                Tick(self.push_stack(value as u8))
            }
        }
    }

    fn inst_lda(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Lda(Exec)),
            Exec => {
                self.reg_a = self.pin_in.data as u32;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
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
                self.reg_x = self.pin_in.data as u32;
                self.flag_s = self.reg_x;
                self.flag_z = self.reg_x;
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
                self.reg_y = self.pin_in.data as u32;
                self.flag_s = self.reg_y;
                self.flag_z = self.reg_y;
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
        self.reg_a = self.lsr(self.reg_a as u8) as u32;

        ExecResult::Done
    }

    fn lsr(&mut self, value: u8) -> u8 {
        self.flag_c = (value as u32) & 1;
        let value = value >> 1;
        self.flag_s = value as u32;
        self.flag_z = value as u32;

        value
    }

    fn inst_nop(&mut self) -> ExecResult {
        ExecResult::Done
    }

    fn inst_ora(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Ora(Exec)),
            Exec => {
                self.reg_a = (self.reg_a | self.pin_in.data as u32) & 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Done
            }
        }
    }

    fn inst_pha(&mut self) -> ExecResult {
        ExecResult::Tick(self.push_stack(self.reg_a as u8))
    }

    fn inst_php(&mut self) -> ExecResult {
        let value = self.reg_p() as u8 | 0x30;
        ExecResult::Tick(self.push_stack(value))
    }

    fn inst_pla(&mut self, step: DummyReadExec) -> ExecResult {
        use DummyReadExec::*;
        use ExecResult::*;
        match step {
            Dummy => {
                let dummy_addr = self.reg_sp | 0x100;
                Next(TickResult::Read(dummy_addr as u16), Instruction::Pla(Read))
            }
            Read => Next(self.pop_stack(), Instruction::Pla(Exec)),
            Exec => {
                self.reg_a = self.pin_in.data as u32;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Done
            }
        }
    }

    fn inst_plp(&mut self, step: DummyReadExec) -> ExecResult {
        use DummyReadExec::*;
        use ExecResult::*;
        match step {
            Dummy => {
                let dummy_addr = self.reg_sp | 0x100;
                Next(TickResult::Read(dummy_addr as u16), Instruction::Plp(Read))
            }
            Read => Next(self.pop_stack(), Instruction::Plp(Exec)),
            Exec => {
                let value = self.pin_in.data as u32;
                if self.flag_i == 1 && value & 0x04 == 0 {
                    self.irq_delay = 1;
                }
                if self.flag_i == 0 && value & 0x04 != 0 {
                    self.irq_set_delay = 1;
                }
                self.set_reg_p(value);
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
        self.reg_a = self.rol(self.reg_a as u8) as u32;

        ExecResult::Done
    }

    fn rol(&mut self, value: u8) -> u8 {
        let value = value as u32;
        let c = if self.flag_c != 0 { 1 } else { 0 };
        self.flag_c = value >> 7 & 1;
        let value = (value << 1 | c) & 0xff;
        self.flag_s = value;
        self.flag_z = value;

        value as u8
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
        self.reg_a = self.ror(self.reg_a as u8) as u32;

        ExecResult::Done
    }

    fn ror(&mut self, value: u8) -> u8 {
        let value = value as u32;
        let c = if self.flag_c != 0 { 0x80 } else { 0 };
        self.flag_c = value & 1;
        let value = (value >> 1 | c) & 0xff;
        self.flag_s = value;
        self.flag_z = value;

        value as u8
    }

    fn inst_rti(&mut self, step: Rti) -> ExecResult {
        use ExecResult::*;
        use Rti::*;
        match step {
            Dummy => {
                let dummy_addr = self.reg_sp | 0x100;
                Next(
                    TickResult::Read(dummy_addr as u16),
                    Instruction::Rti(ReadRegP),
                )
            }
            ReadRegP => Next(self.pop_stack(), Instruction::Rti(ReadRegPcLow)),
            ReadRegPcLow => {
                let reg_p = self.pin_in.data;
                self.set_reg_p(reg_p as u32);
                Next(self.pop_stack(), Instruction::Rti(ReadRegPcHigh))
            }
            ReadRegPcHigh => {
                let low_value = self.pin_in.data;
                Next(self.pop_stack(), Instruction::Rti(Exec(low_value as u16)))
            }
            Exec(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                self.reg_pc = (high_addr | low_addr) as u32;
                Done
            }
        }
    }

    fn inst_rts(&mut self, step: Rts) -> ExecResult {
        use ExecResult::*;
        use Rts::*;
        match step {
            Dummy => {
                let dummy_addr = self.reg_sp | 0x100;
                Next(
                    TickResult::Read(dummy_addr as u16),
                    Instruction::Rts(ReadRegPcLow),
                )
            }
            ReadRegPcLow => Next(self.pop_stack(), Instruction::Rts(ReadRegPcHigh)),
            ReadRegPcHigh => {
                let low_value = self.pin_in.data as u16;
                Next(self.pop_stack(), Instruction::Rts(Exec(low_value)))
            }
            Exec(low_addr) => {
                let high_addr = (self.pin_in.data as u16) << 8;
                self.reg_pc = (high_addr | low_addr).wrapping_add(1) as u32;
                Tick(TickResult::Read(self.reg_pc as u16))
            }
        }
    }

    fn inst_sbc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::Sbc(Exec)),
            Exec => {
                let value = self.pin_in.data as i32;
                let temp_a = self.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.flag_c as i32 - 1));
                self.flag_v = (((temp_a ^ value) & (temp_a ^ temp)) >> 7) as u32 & 1;
                self.flag_c = if temp < 0 { 0 } else { 1 };
                self.reg_a = (temp as u32) & 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Done
            }
        }
    }

    fn inst_sec(&mut self) -> ExecResult {
        self.flag_c = 1;
        ExecResult::Done
    }

    fn inst_sed(&mut self) -> ExecResult {
        self.flag_d = 1;
        ExecResult::Done
    }

    fn inst_sei(&mut self) -> ExecResult {
        if self.flag_i == 0 {
            self.irq_set_delay = 1;
        }
        self.flag_i = 1;
        ExecResult::Done
    }

    fn inst_sta(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.reg_a as u8))
    }

    fn inst_stx(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.reg_x as u8))
    }

    fn inst_sty(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Write(addr, self.reg_y as u8))
    }

    fn inst_tax(&mut self) -> ExecResult {
        self.reg_x = self.reg_a;
        self.flag_s = self.reg_x;
        self.flag_z = self.reg_x;
        ExecResult::Done
    }

    fn inst_tay(&mut self) -> ExecResult {
        self.reg_y = self.reg_a;
        self.flag_s = self.reg_y;
        self.flag_z = self.reg_y;
        ExecResult::Done
    }

    fn inst_tsx(&mut self) -> ExecResult {
        self.reg_x = self.reg_sp;
        self.flag_s = self.reg_x;
        self.flag_z = self.reg_x;
        ExecResult::Done
    }

    fn inst_txa(&mut self) -> ExecResult {
        self.reg_a = self.reg_x;
        self.flag_s = self.reg_a;
        self.flag_z = self.reg_a;
        ExecResult::Done
    }

    fn inst_txs(&mut self) -> ExecResult {
        self.reg_sp = self.reg_x;
        ExecResult::Done
    }

    fn inst_tya(&mut self) -> ExecResult {
        self.reg_a = self.reg_y;
        self.flag_s = self.reg_a;
        self.flag_z = self.reg_a;
        ExecResult::Done
    }

    fn ill_inst_ahx(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Read(addr))
    }

    fn ill_inst_alr(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllAlr(Exec)),
            Exec => {
                let value = self.pin_in.data as u32;
                self.reg_a &= value;
                self.flag_c = self.reg_a & 1;
                self.reg_a >>= 1;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
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
                let value = self.pin_in.data as u32;
                self.reg_a &= value;
                self.flag_c = (self.reg_a >> 7) & 1;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
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
                let value = self.pin_in.data as u32;
                self.reg_a &= value;
                if self.flag_c != 0 {
                    self.flag_c = self.reg_a & 1;
                    self.reg_a = ((self.reg_a >> 1) | 0x80) & 0xff;
                    self.flag_s = self.reg_a;
                    self.flag_z = self.reg_a;
                } else {
                    self.flag_c = self.reg_a & 1;
                    self.reg_a = (self.reg_a >> 1) & 0xff;
                    self.flag_s = self.reg_a;
                    self.flag_z = self.reg_a;
                }
                match ((self.reg_a & 0x40), (self.reg_a & 0x20)) {
                    (0, 0) => {
                        self.flag_c = 0;
                        self.flag_v = 0;
                    }
                    (_, 0) => {
                        self.flag_c = 1;
                        self.flag_v = 1;
                    }
                    (0, _) => {
                        self.flag_c = 0;
                        self.flag_v = 1;
                    }
                    (_, _) => {
                        self.flag_c = 1;
                        self.flag_v = 0;
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
                let value = self.pin_in.data as u32;
                self.reg_x &= self.reg_a;
                let temp = self.reg_x.wrapping_sub(value);
                self.flag_c = if temp > self.reg_x { 0 } else { 1 };
                self.reg_x = temp & 0xff;
                self.flag_s = self.reg_x;
                self.flag_z = self.reg_x;
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
                let value = data.wrapping_sub(1) as u32;
                self.flag_c = if self.reg_a >= value { 1 } else { 0 };
                self.flag_z = if self.reg_a == value { 0 } else { 1 };
                self.flag_s = self.reg_a.wrapping_sub(value) & 0xff;
                Tick(TickResult::Write(addr, value as u8))
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
                let temp_a = self.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.flag_c as i32 - 1));
                self.flag_v = (((temp_a ^ value) & (temp_a ^ temp)) >> 7) as u32 & 1;
                self.flag_c = if temp < 0 { 0 } else { 1 };
                self.reg_a = (temp as u32) & 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Tick(TickResult::Write(addr, value as u8))
            }
        }
    }

    fn ill_inst_kil(&mut self) -> ExecResult {
        eprintln!("KIL encountered");
        ExecResult::Done
    }

    fn ill_inst_las(&mut self, addr: u16) -> ExecResult {
        ExecResult::Tick(TickResult::Read(addr))
    }

    fn ill_inst_lax(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllLax(Exec)),
            Exec => {
                self.reg_a = self.pin_in.data as u32;
                self.reg_x = self.reg_a;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Done
            }
        }
    }

    fn ill_inst_nop(&mut self) -> ExecResult {
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
                let c = if self.flag_c != 0 { 1 } else { 0 };
                self.flag_c = (data as u32) >> 7 & 1;
                let value = ((data as u32) << 1 | c) & 0xff;
                self.reg_a &= value;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Tick(TickResult::Write(addr, value as u8))
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
                let c = if self.flag_c != 0 { 0x80 } else { 0 };
                self.flag_c = data & 1;
                let data = (data >> 1 | c) & 0xff;
                let value = self.reg_a.wrapping_add(data.wrapping_add(self.flag_c));
                self.flag_v = ((!(self.reg_a ^ data) & (self.reg_a ^ value)) >> 7) & 1;
                self.flag_c = if value > 0xff { 1 } else { 0 };
                self.reg_a = value & 0xff;
                self.flag_s = value & 0xff;
                self.flag_z = value & 0xff;

                Tick(TickResult::Write(addr, data as u8))
            }
        }
    }

    fn ill_inst_sax(&mut self, addr: u16) -> ExecResult {
        let value = (self.reg_a & self.reg_x) & 0xff;
        ExecResult::Tick(TickResult::Write(addr, value as u8))
    }

    fn ill_inst_sbc(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllSbc(Exec)),
            Exec => {
                let value = self.pin_in.data as i32;
                let temp_a = self.reg_a as i32;
                let temp = temp_a.wrapping_sub(value.wrapping_sub(self.flag_c as i32 - 1));
                self.flag_v = (((temp_a ^ value) & (temp_a ^ temp)) >> 7) as u32 & 1;
                self.flag_c = if temp < 0 { 0 } else { 1 };
                self.reg_a = (temp as u32) & 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Done
            }
        }
    }

    fn ill_inst_shx(&mut self, addr: u16) -> ExecResult {
        let temp_addr = addr as u32;
        let value = (self.reg_x & ((temp_addr >> 8).wrapping_add(1))) & 0xff;
        let temp = temp_addr.wrapping_sub(self.reg_y) & 0xff;
        if self.reg_y.wrapping_add(temp) <= 0xff {
            ExecResult::Tick(TickResult::Write(addr, value as u8))
        } else {
            // let value = self.bus.peek(system, state, addr);
            ExecResult::Tick(TickResult::Write(addr, value as u8))
        }
    }

    fn ill_inst_shy(&mut self, addr: u16) -> ExecResult {
        let temp_addr = addr as u32;
        let value = (self.reg_y & ((temp_addr >> 8).wrapping_add(1))) & 0xff;
        let temp = temp_addr.wrapping_sub(self.reg_x) & 0xff;
        if self.reg_x.wrapping_add(temp) <= 0xff {
            ExecResult::Tick(TickResult::Write(addr, value as u8))
        } else {
            // let value = self.bus.peek(system, state, addr);
            ExecResult::Tick(TickResult::Write(addr, value as u8))
        }
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
                let value = data as u32;
                self.flag_c = (value >> 7) & 1;
                let value = (value << 1) & 0xff;
                self.reg_a |= value;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Tick(TickResult::Write(addr, value as u8))
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
                let value = data as u32;
                self.flag_c = value & 1;
                let value = value >> 1;
                self.reg_a ^= value;
                self.reg_a &= 0xff;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
                Tick(TickResult::Write(addr, value as u8))
            }
        }
    }

    fn ill_inst_tas(&mut self, addr: u16) -> ExecResult {
        self.reg_sp = self.reg_x & self.reg_a;
        let value = self.reg_sp & ((addr as u32) >> 8);
        ExecResult::Tick(TickResult::Write(addr, value as u8))
    }

    fn ill_inst_xaa(&mut self, addr: u16, step: ReadExec) -> ExecResult {
        use ExecResult::*;
        use ReadExec::*;
        match step {
            Read => Next(TickResult::Read(addr), Instruction::IllXaa(Exec)),
            Exec => {
                let value = self.pin_in.data as u32;
                self.reg_a = self.reg_x & value;
                self.flag_s = self.reg_a;
                self.flag_z = self.reg_a;
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
