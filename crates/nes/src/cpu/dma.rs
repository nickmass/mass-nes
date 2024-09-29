#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use super::{CpuPinIn, TickResult};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum DmcDma {
    Dummy,
    Read,
    Complete,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum DmcDmaKind {
    Load(u16),
    Reload(u16),
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum OamDma {
    Read,
    Write,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
enum Alignment {
    Get,
    Put,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Dma {
    cycle: u64,
    dmc_timer: Option<(u64, u16)>,
    want_dmc: Option<u16>,
    want_oam: Option<u16>,
    dmc_step: Option<DmcDma>,
    oam_step: Option<OamDma>,
    halt_addr: Option<u16>,
    dmc_sample: Option<u8>,
    oam_offset: u16,
    oam_active: bool,
    oam_read_cycle: u64,
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            cycle: 0,
            dmc_timer: None,
            want_dmc: None,
            want_oam: None,
            dmc_step: None,
            oam_step: None,
            halt_addr: None,
            dmc_sample: None,
            oam_offset: 0,
            oam_active: false,
            oam_read_cycle: 0,
        }
    }

    pub fn tick(&mut self, pin_in: CpuPinIn) -> Option<TickResult> {
        self.cycle += 1;

        if let Some((timer, addr)) = self.dmc_timer {
            if timer == 0 {
                self.want_dmc = Some(addr);
                self.dmc_timer = None;
            } else {
                self.dmc_timer = Some((timer - 1, addr));
            }
        }

        if let Some(halt_addr) = self.halt_addr {
            if let Some(tick) = self.dmc(pin_in) {
                return Some(tick);
            }

            if let Some(tick) = self.oam(pin_in) {
                return Some(tick);
            }

            if self.want_dmc.or(self.want_oam).is_none() {
                self.halt_addr.take().map(TickResult::Read)
            } else {
                Some(TickResult::Idle(halt_addr))
            }
        } else {
            None
        }
    }

    pub fn try_halt(&mut self, tick: TickResult) -> Option<TickResult> {
        match (self.halt_addr, self.want_dmc.or(self.want_oam)) {
            (None, Some(_)) => match tick {
                TickResult::Read(addr) => {
                    self.halt_addr = Some(addr);
                    self.halt_addr.map(TickResult::Read)
                }
                TickResult::Write(_, _) | TickResult::Idle(_) => None,
            },
            (Some(_), Some(_)) => self.halt_addr.map(TickResult::Idle),
            (Some(_), None) => self.halt_addr.take().map(TickResult::Read),
            (None, None) => None,
        }
    }

    // the specific orientation of Get/Put has major effects on test roms,
    // I think the issue is in my APU tick implmentation being 1:1 with the
    // cpu ticks instead of being split into even/odd halves
    fn alignment(&self) -> Alignment {
        if self.cycle & 1 == 0 {
            Alignment::Put
        } else {
            Alignment::Get
        }
    }

    pub fn dmc_sample(&mut self) -> Option<u8> {
        self.dmc_sample.take()
    }

    pub fn request_dmc_dma(&mut self, dma: DmcDmaKind) {
        match (dma, self.alignment()) {
            (DmcDmaKind::Load(addr), Alignment::Get) => self.dmc_timer = Some((3, addr)),
            (DmcDmaKind::Load(addr), Alignment::Put) => self.dmc_timer = Some((2, addr)),
            (DmcDmaKind::Reload(addr), _) => self.want_dmc = Some(addr),
        }
    }

    pub fn request_oam_dma(&mut self, high_addr: u16) {
        self.want_oam = Some(high_addr)
    }

    fn dmc(&mut self, pin_in: CpuPinIn) -> Option<TickResult> {
        let &Some(addr) = &self.want_dmc else {
            return None;
        };

        let step = if let Some(step) = self.dmc_step {
            step
        } else {
            DmcDma::Dummy
        };

        match (step, self.alignment()) {
            (DmcDma::Dummy, _) => {
                self.dmc_step = Some(DmcDma::Read);
                None
            }
            (DmcDma::Read, Alignment::Put) => None,
            (DmcDma::Read, Alignment::Get) => {
                self.dmc_step = Some(DmcDma::Complete);
                Some(TickResult::Read(addr))
            }
            (DmcDma::Complete, _) => {
                self.dmc_sample = Some(pin_in.data);
                self.want_dmc = None;
                self.dmc_step = None;
                None
            }
        }
    }

    fn oam(&mut self, pin_in: CpuPinIn) -> Option<TickResult> {
        let &Some(addr) = &self.want_oam else {
            return None;
        };

        let step = if let Some(step) = self.oam_step {
            step
        } else {
            self.oam_offset = 0;
            self.oam_active = true;
            OamDma::Read
        };

        match (step, self.alignment()) {
            (OamDma::Read, Alignment::Put) | (OamDma::Write, Alignment::Get) => None,
            (OamDma::Read, Alignment::Get) => {
                if !self.oam_active {
                    self.want_oam = None;
                    self.oam_step = None;
                    None
                } else {
                    self.oam_read_cycle = self.cycle;
                    let read_addr = addr << 8 | self.oam_offset & 0xff;
                    self.oam_step = Some(OamDma::Write);
                    Some(TickResult::Read(read_addr))
                }
            }
            (OamDma::Write, Alignment::Put) => {
                self.oam_step = Some(OamDma::Read);
                if self.cycle == self.oam_read_cycle + 1 {
                    self.oam_offset += 1;
                    if self.oam_offset > 0xff {
                        self.oam_active = false;
                    }
                    Some(TickResult::Write(0x2004, pin_in.data))
                } else {
                    None
                }
            }
        }
    }
}
