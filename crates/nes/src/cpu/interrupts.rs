#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use super::{CpuPinIn, CpuRegs, TickResult};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Triggered {
    Irq,
    Nmi,
    Reset,
    Power,
}

impl Triggered {
    pub fn vector(&self) -> u16 {
        match self {
            Triggered::Reset | Triggered::Power => 0xfffc,
            Triggered::Nmi => 0xfffa,
            Triggered::Irq => 0xfffe,
        }
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Irq {
    ReadPcOne,
    ReadPcTwo,
    WriteRegPcHigh,
    WriteRegPcLow,
    WriteRegP,
    ReadHighJump,
    ReadLowJump(u16),
    UpdateRegPc,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Power {
    ReadRegPcLow,
    ReadRegPcHigh(u16),
    UpdateRegPc(u16),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Steps {
    Irq(Irq),
    Reset(Power),
    Power(Power),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Interrupts {
    nmi: NmiInterrupt,
    irq: IrqInterrupt,
    reset: ResetInterrupt,
    power: PowerInterrupt,
    triggered: Option<Triggered>,
    step: Option<Steps>,
    power_up_pc: Option<u16>,
}

impl Interrupts {
    pub fn new() -> Self {
        Self {
            nmi: NmiInterrupt::new(),
            irq: IrqInterrupt::new(),
            reset: ResetInterrupt::new(),
            power: PowerInterrupt::new(),
            triggered: None,
            step: None,
            power_up_pc: None,
        }
    }

    pub fn with_power_up_pc(&mut self, pc: u16) {
        self.power_up_pc = Some(pc);
    }

    pub fn tick(&mut self, pin_in: &CpuPinIn) {
        self.power.tick(pin_in);
        self.reset.tick(pin_in);
        self.nmi.tick(pin_in);
        self.irq.tick(pin_in);
    }

    pub fn triggered(&mut self) -> Option<Triggered> {
        self.triggered.take()
    }

    pub fn poll(&mut self, regs: &CpuRegs) {
        if self.irq.poll(regs) {
            self.triggered = Some(Triggered::Irq).max(self.triggered);
        }
        if self.nmi.poll() {
            self.triggered = Some(Triggered::Nmi).max(self.triggered);
        }
        if self.reset.poll() {
            self.triggered = Some(Triggered::Reset).max(self.triggered);
        }
        if self.power.poll() {
            self.triggered = Some(Triggered::Power).max(self.triggered);
        }
    }

    pub fn interrupt(&mut self, pin_in: &CpuPinIn, regs: &mut CpuRegs) -> Option<TickResult> {
        let step = if let Some(step) = self.step.take() {
            step
        } else if let Some(trigger) = &self.triggered {
            match trigger {
                Triggered::Power => Steps::Power(Power::ReadRegPcLow),
                Triggered::Reset => Steps::Reset(Power::ReadRegPcLow),
                Triggered::Nmi => Steps::Irq(Irq::ReadPcOne),
                Triggered::Irq => Steps::Irq(Irq::ReadPcOne),
            }
        } else {
            return None;
        };

        match step {
            Steps::Irq(irq) => self.irq_step(irq, pin_in, regs),
            Steps::Reset(reset) => self.reset_step(reset, pin_in, regs),
            Steps::Power(power) => self.power_step(power, pin_in, regs),
        }
    }

    fn irq_step(&mut self, step: Irq, pin_in: &CpuPinIn, regs: &mut CpuRegs) -> Option<TickResult> {
        let next_step;
        let tick = match step {
            Irq::ReadPcOne => {
                next_step = Irq::ReadPcTwo;
                TickResult::Read(regs.reg_pc)
            }
            Irq::ReadPcTwo => {
                next_step = Irq::WriteRegPcHigh;
                TickResult::Read(regs.reg_pc)
            }
            Irq::WriteRegPcHigh => {
                next_step = Irq::WriteRegPcLow;
                let val = (regs.reg_pc >> 8) & 0xff;
                regs.push_stack(val as u8)
            }
            Irq::WriteRegPcLow => {
                next_step = Irq::WriteRegP;
                let val = regs.reg_pc & 0xff;
                regs.push_stack(val as u8)
            }
            Irq::WriteRegP => {
                next_step = Irq::ReadHighJump;
                let val = regs.reg_p() | 0x20;
                regs.push_stack(val)
            }
            Irq::ReadHighJump => {
                self.poll(regs);
                let triggered = self.triggered.take().expect("was triggered");
                let addr = triggered.vector();
                next_step = Irq::ReadLowJump(addr);
                TickResult::Read(addr)
            }
            Irq::ReadLowJump(addr) => {
                next_step = Irq::UpdateRegPc;
                regs.reg_pc &= 0xff00;
                regs.reg_pc |= pin_in.data as u16;
                regs.flag_i = true;
                TickResult::Read(addr + 1)
            }
            Irq::UpdateRegPc => {
                regs.reg_pc &= 0x00ff;
                regs.reg_pc |= (pin_in.data as u16) << 8;
                return None;
            }
        };

        self.step = Some(Steps::Irq(next_step));

        Some(tick)
    }

    fn reset_step(
        &mut self,
        step: Power,
        pin_in: &CpuPinIn,
        regs: &mut CpuRegs,
    ) -> Option<TickResult> {
        let next_step;
        let tick = match step {
            Power::ReadRegPcLow => {
                let trigger = self.triggered.take().unwrap_or(Triggered::Reset);
                let vector = trigger.vector();
                next_step = Power::ReadRegPcHigh(vector);
                TickResult::Read(vector)
            }
            Power::ReadRegPcHigh(vector) => {
                next_step = Power::UpdateRegPc(pin_in.data as u16);
                TickResult::Read(vector + 1)
            }
            Power::UpdateRegPc(low_addr) => {
                let high_addr = (pin_in.data as u16) << 8;
                regs.reg_pc = low_addr | high_addr;
                regs.reg_sp = regs.reg_sp.wrapping_sub(3);
                regs.flag_i = true;
                return None;
            }
        };

        self.step = Some(Steps::Reset(next_step));

        Some(tick)
    }

    fn power_step(
        &mut self,
        step: Power,
        pin_in: &CpuPinIn,
        regs: &mut CpuRegs,
    ) -> Option<TickResult> {
        let next_step;
        let tick = match step {
            Power::ReadRegPcLow => {
                let trigger = self.triggered.take().unwrap_or(Triggered::Power);
                let vector = trigger.vector();
                next_step = Power::ReadRegPcHigh(vector);
                TickResult::Read(vector)
            }
            Power::ReadRegPcHigh(vector) => {
                next_step = Power::UpdateRegPc(pin_in.data as u16);
                TickResult::Read(vector + 1)
            }
            Power::UpdateRegPc(low_addr) => {
                let high_addr = (pin_in.data as u16) << 8;
                regs.reg_pc = low_addr | high_addr;
                regs.set_reg_p(0x34);
                regs.reg_sp = 0xfd;
                if let Some(addr) = self.power_up_pc {
                    regs.reg_pc = addr;
                }
                return None;
            }
        };

        self.step = Some(Steps::Power(next_step));

        Some(tick)
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct IrqInterrupt {
    triggered: bool,
}

impl IrqInterrupt {
    fn new() -> Self {
        Self { triggered: false }
    }

    fn tick(&mut self, pin_in: &CpuPinIn) {
        self.triggered = pin_in.irq;
    }

    fn poll(&mut self, regs: &CpuRegs) -> bool {
        !regs.flag_i && self.triggered
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct NmiInterrupt {
    was_nmi: bool,
    triggered: bool,
}

impl NmiInterrupt {
    fn new() -> Self {
        Self {
            was_nmi: false,
            triggered: false,
        }
    }

    fn tick(&mut self, pin_in: &CpuPinIn) {
        if !self.was_nmi && pin_in.nmi {
            self.triggered = true;
        }

        self.was_nmi = pin_in.nmi;
    }

    fn poll(&mut self) -> bool {
        let trigged = self.triggered;
        self.triggered = false;
        trigged
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct ResetInterrupt {
    was_reset: bool,
    triggered: bool,
}

impl ResetInterrupt {
    fn new() -> Self {
        Self {
            was_reset: false,
            triggered: false,
        }
    }

    fn tick(&mut self, pin_in: &CpuPinIn) {
        if !self.was_reset && pin_in.reset {
            self.triggered = true;
        }

        self.was_reset = pin_in.reset;
    }

    fn poll(&mut self) -> bool {
        let trigged = self.triggered;
        self.triggered = false;
        trigged
    }
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
struct PowerInterrupt {
    was_power: bool,
    triggered: bool,
}

impl PowerInterrupt {
    fn new() -> Self {
        Self {
            was_power: false,
            triggered: false,
        }
    }

    fn tick(&mut self, pin_in: &CpuPinIn) {
        if !self.was_power && pin_in.power {
            self.triggered = true;
        }

        self.was_power = pin_in.power;
    }

    fn poll(&mut self) -> bool {
        let trigged = self.triggered;
        self.triggered = false;
        trigged
    }
}
