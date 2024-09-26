#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::apu::ApuSnapshot;
use crate::bus::{AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::Channel;

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Default)]
pub struct Triangle {
    timer_counter: u16,
    linear_counter: u8,
    linear_reload: bool,
    length_counter: u8,
    sequencer: u32,
    enabled: bool,
    regs: [u8; 4],
    current_tick: u64,
    forced_clock: bool,
}

impl Triangle {
    pub fn new() -> Triangle {
        Triangle {
            ..Default::default()
        }
    }

    pub fn forced_clock(&mut self) {
        self.forced_clock = true;
    }

    fn timer_load(&self) -> u16 {
        (self.regs[2] as u16) | ((self.regs[3] as u16 & 7) << 8)
    }

    fn linear_load(&self) -> u8 {
        self.regs[0] & 0x7f
    }

    fn length_load(&self) -> u8 {
        if !self.enabled {
            0
        } else {
            crate::apu::LENGTH_TABLE[(self.regs[3] >> 3 & 0x1f) as usize]
        }
    }

    fn halt(&self) -> bool {
        self.regs[0] & 0x80 != 0
    }

    fn sequence(&self) -> u8 {
        let table = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15,
        ];

        table[(self.sequencer % 32) as usize]
    }
}

impl Channel for Triangle {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_write(DeviceKind::Triangle, AndEqualsAndMask(0xfffc, 0x4008, 0x3));
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.regs[addr as usize] = value;
        match addr {
            0 => {}
            1 => {}
            2 => {}
            3 => {
                self.length_counter = self.length_load();
                self.linear_reload = true;
            }
            _ => unreachable!(),
        }
    }

    fn tick(&mut self, state: ApuSnapshot) -> u8 {
        self.current_tick += 1;

        if self.timer_counter == 0 {
            self.timer_counter = self.timer_load();
            if self.length_counter != 0 && self.linear_counter != 0 {
                self.sequencer = self.sequencer.wrapping_add(1);
            }
        } else {
            self.timer_counter -= 1;
        }

        if (state.is_quarter_frame || self.forced_clock) && self.current_tick & 1 == 0 {
            if self.linear_reload {
                self.linear_counter = self.linear_load();
            } else if self.linear_counter != 0 {
                self.linear_counter -= 1;
            }
            if !self.halt() {
                self.linear_reload = false;
            }
        }

        if (state.is_half_frame || self.forced_clock) && self.length_counter != 0 && !self.halt() {
            self.length_counter -= 1;
        }

        self.forced_clock = false;

        self.sequence()
    }

    fn enable(&mut self) {
        self.enabled = true;
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
    }

    fn get_state(&self) -> bool {
        self.length_counter > 0
    }
}
