use std::rc::Rc;

use crate::debug::{Debug, DebugEvent};

#[cfg(feature = "save-states")]
use nes_traits::SaveState;
#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum VrcIrqMode {
    Cycle,
    Scanline,
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Clone)]
pub struct VrcIrq {
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    counter: u8,
    scanline_counter: i16,
    latch: u8,
    mode: VrcIrqMode,
    enabled: bool,
    renable: bool,
    triggered: bool,
}

impl VrcIrq {
    pub fn new(debug: Rc<Debug>) -> Self {
        Self {
            debug,
            counter: 0,
            scanline_counter: 341,
            latch: 0,
            mode: VrcIrqMode::Cycle,
            enabled: false,
            renable: false,
            triggered: false,
        }
    }

    pub fn tick(&mut self) {
        self.scanline_counter -= 3;
        match self.mode {
            VrcIrqMode::Cycle => {
                self.scanline_counter += 341;
                self.trigger();
            }
            VrcIrqMode::Scanline => {
                if self.scanline_counter <= 0 {
                    self.scanline_counter += 341;
                    self.trigger();
                }
            }
        }
    }

    fn trigger(&mut self) {
        if self.counter == 0xff {
            if self.enabled {
                if !self.triggered {
                    self.debug.event(DebugEvent::MapperIrq);
                }
                self.triggered = true;
            }
            self.counter = self.latch;
            self.enabled = self.renable;
        } else {
            self.counter += 1;
        }
    }

    pub fn irq(&self) -> bool {
        self.triggered
    }

    pub fn latch(&mut self, value: u8) {
        self.latch = value;
    }

    pub fn latch_lo(&mut self, value: u8) {
        self.latch = (self.latch & 0xf0) | (value & 0x0f);
    }

    pub fn latch_hi(&mut self, value: u8) {
        self.latch = (self.latch & 0x0f) | ((value & 0x0f) << 4);
    }

    pub fn control(&mut self, value: u8) {
        self.renable = value & 0x1 != 0;
        self.enabled = value & 0x2 != 0;
        self.mode = if value & 0x4 != 0 {
            VrcIrqMode::Cycle
        } else {
            VrcIrqMode::Scanline
        };

        self.triggered = false;
        self.scanline_counter = 341;

        if self.enabled {
            self.counter = self.latch;
        }
    }

    pub fn acknowledge(&mut self) {
        self.triggered = false;
    }
}
