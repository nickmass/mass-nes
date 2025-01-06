#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use super::TickResult;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Default)]
pub struct CpuRegs {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u8,
    pub flag_c: bool,
    pub flag_z: bool,
    pub flag_i: bool,
    pub flag_d: bool,
    pub flag_v: bool,
    pub flag_s: bool,
}

impl CpuRegs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reg_p(&self) -> u8 {
        let mut val = 0;
        if self.flag_c {
            val |= 0x01;
        }
        if self.flag_z {
            val |= 0x02;
        }
        if self.flag_i {
            val |= 0x04;
        }
        if self.flag_d {
            val |= 0x08;
        }
        if self.flag_v {
            val |= 0x40;
        }
        if self.flag_s {
            val |= 0x80;
        }

        val
    }

    pub fn set_reg_p(&mut self, value: u8) {
        self.flag_c = value & 0x01 != 0;
        self.flag_z = value & 0x02 != 0;
        self.flag_i = value & 0x04 != 0;
        self.flag_d = value & 0x08 != 0;
        self.flag_v = value & 0x40 != 0;
        self.flag_s = value & 0x80 != 0;
    }

    pub fn read_pc(&mut self) -> TickResult {
        let pc = self.reg_pc;
        self.reg_pc = pc.wrapping_add(1);
        TickResult::Read(pc)
    }

    pub fn fetch_pc(&mut self) -> TickResult {
        let pc = self.reg_pc;
        self.reg_pc = pc.wrapping_add(1);
        TickResult::Fetch(pc)
    }

    pub fn pop_stack(&mut self) -> TickResult {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        let addr = self.reg_sp as u16 | 0x100;
        TickResult::Read(addr)
    }

    pub fn push_stack(&mut self, value: u8) -> TickResult {
        let addr = self.reg_sp as u16 | 0x100;
        self.reg_sp = self.reg_sp.wrapping_sub(1) & 0xff;
        TickResult::Write(addr, value)
    }

    pub fn set_flags_zs(&mut self, value: u8) {
        self.flag_z = value == 0;
        self.flag_s = value & 0x80 != 0;
    }
}
