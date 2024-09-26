#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::apu::ApuSnapshot;
use crate::bus::{AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::Channel;

//Im not sure if these are in CPU clocks, or APU clocks.
const RATES: &[u16] = &[
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Default)]
pub struct Noise {
    timer_counter: u16,
    length_counter: u8,
    enabled: bool,
    shifter: u16,
    envelope_start: bool,
    envelope_divider: u8,
    decay_counter: u8,
    regs: [u8; 4],
    current_tick: u64,
    forced_clock: bool,
}

impl Noise {
    pub fn new() -> Noise {
        Noise {
            shifter: 1,
            ..Default::default()
        }
    }

    pub fn forced_clock(&mut self) {
        self.forced_clock = true;
    }

    fn length_load(&self) -> u8 {
        if !self.enabled {
            0
        } else {
            crate::apu::LENGTH_TABLE[(self.regs[3] >> 3 & 0x1f) as usize]
        }
    }

    fn envelope_volume(&self) -> u8 {
        self.regs[0] & 0xf
    }

    fn envelope_output(&self) -> u8 {
        if self.constant_volume() {
            self.envelope_volume()
        } else {
            self.decay_counter
        }
    }

    fn constant_volume(&self) -> bool {
        self.regs[0] & 0x10 != 0
    }
    fn halt(&self) -> bool {
        self.regs[0] & 0x20 != 0
    }

    fn noise_mode(&self) -> bool {
        self.regs[2] & 0x80 != 0
    }

    fn clock_shifter(&mut self) {
        let feedback = if self.noise_mode() {
            (self.shifter & 1) ^ ((self.shifter >> 6) & 1)
        } else {
            (self.shifter & 1) ^ ((self.shifter >> 1) & 1)
        };

        self.shifter >>= 1;
        self.shifter |= feedback << 14;
    }

    fn timer_period(&self) -> u16 {
        RATES[(self.regs[2] & 0xf) as usize]
    }
}

impl Channel for Noise {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_write(DeviceKind::Noise, AndEqualsAndMask(0xfffc, 0x400c, 0x3));
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.regs[addr as usize] = value;
        match addr {
            0 => {}
            1 => {}
            2 => {}
            3 => {
                self.length_counter = self.length_load();
                self.envelope_start = true;
            }
            _ => unreachable!(),
        }
    }

    fn tick(&mut self, state: ApuSnapshot) -> u8 {
        self.current_tick += 1;

        if self.current_tick & 1 == 0 {
            if self.timer_counter == 0 {
                self.timer_counter = self.timer_period();
                self.clock_shifter();
            } else {
                self.timer_counter -= 1;
            }
        }

        if state.is_quarter_frame || self.forced_clock {
            if self.envelope_start {
                self.envelope_start = false;
                self.decay_counter = 0xf;
                self.envelope_divider = self.envelope_volume();
            } else if self.envelope_divider == 0 {
                self.envelope_divider = self.envelope_volume();
                if self.decay_counter == 0 {
                    if self.halt() {
                        self.decay_counter = 0xf
                    }
                } else {
                    self.decay_counter -= 1;
                }
            } else {
                self.envelope_divider -= 1;
            }
        }

        if (state.is_half_frame || self.forced_clock) && self.length_counter != 0 && !self.halt() {
            self.length_counter -= 1;
        }

        self.forced_clock = false;

        if (self.shifter & 1) == 1 || self.length_counter == 0 {
            0
        } else {
            self.envelope_output()
        }
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
