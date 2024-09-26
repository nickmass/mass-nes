use nes_traits::SaveState;
use serde::{Deserialize, Serialize};

use crate::apu::ApuSnapshot;
use crate::bus::{AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::Channel;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PulseChannel {
    InternalOne,
    InternalTwo,
}

impl Default for PulseChannel {
    fn default() -> PulseChannel {
        PulseChannel::InternalOne
    }
}

#[derive(Default, SaveState)]
pub struct Pulse {
    channel: PulseChannel,
    period: u16,
    timer_counter: u16,
    length_counter: u8,
    sequencer: u8,
    enabled: bool,
    envelope_start: bool,
    envelope_divider: u8,
    decay_counter: u8,
    sweep_reload: bool,
    sweep_divider: u8,
    regs: [u8; 4],
    current_tick: u64,
    forced_clock: bool,
}

impl Pulse {
    pub fn new(channel: PulseChannel) -> Pulse {
        Pulse {
            channel,
            ..Default::default()
        }
    }

    pub fn forced_clock(&mut self) {
        self.forced_clock = true;
    }

    fn timer_load(&self) -> u16 {
        (self.regs[2] as u16) | ((self.regs[3] as u16 & 7) << 8)
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

    fn shift_count(&self) -> u8 {
        self.regs[1] & 0x7
    }

    fn sweep_negate(&self) -> bool {
        self.regs[1] & 0x8 != 0
    }

    fn sweep_load(&self) -> u8 {
        self.regs[1] >> 4 & 0x7
    }

    fn sweep_enabled(&self) -> bool {
        self.regs[1] & 0x80 != 0
    }

    fn sweep_target(&self) -> u16 {
        let mut period = self.period;
        period >>= self.shift_count() as u16;

        let chan = match self.channel {
            PulseChannel::InternalOne => 1,
            PulseChannel::InternalTwo => 0,
        };

        period = if self.sweep_negate() {
            if period != 0 && self.period > period - chan {
                self.period - (period - chan)
            } else {
                self.period
            }
        } else {
            self.period + period
        };

        period
    }

    fn sweep_timer(&self) -> u16 {
        let target = self.sweep_target();
        if self.period < 8 || target > 0x7ff || !self.sweep_enabled() || self.shift_count() == 0 {
            self.period
        } else {
            target
        }
    }

    fn sweep_output(&self) -> u8 {
        let target = self.sweep_target();
        if self.period < 8 || target > 0x7ff {
            0
        } else {
            self.envelope_output()
        }
    }

    fn duty_sequence(&self) -> [bool; 8] {
        match self.regs[0] >> 6 & 3 {
            0 => [false, true, false, false, false, false, false, false],
            1 => [false, true, true, false, false, false, false, false],
            2 => [false, true, true, true, true, false, false, false],
            3 => [true, false, false, true, true, true, true, true],
            _ => unreachable!(),
        }
    }

    fn duty(&self) -> bool {
        self.duty_sequence()[(self.sequencer & 7) as usize]
    }
}

impl Channel for Pulse {
    fn register(&self, cpu: &mut AddressBus) {
        match self.channel {
            PulseChannel::InternalOne => {
                cpu.register_write(DeviceKind::PulseOne, AndEqualsAndMask(0xfffc, 0x4000, 0x3));
            }
            PulseChannel::InternalTwo => {
                cpu.register_write(DeviceKind::PulseTwo, AndEqualsAndMask(0xfffc, 0x4004, 0x3));
            }
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.regs[addr as usize] = value;
        match addr {
            0 => {}
            1 => {
                self.sweep_reload = true;
            }
            2 => {
                self.period = self.timer_load();
            }
            3 => {
                self.period = self.timer_load();
                self.sequencer = 0;
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
                self.timer_counter = self.period;
                self.sequencer = self.sequencer.wrapping_add(1);
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

        if state.is_half_frame || self.forced_clock {
            if self.length_counter != 0 && !self.halt() {
                self.length_counter -= 1;
            }

            if self.sweep_reload {
                if self.sweep_divider == 0 {
                    self.period = self.sweep_timer();
                }
                self.sweep_divider = self.sweep_load();
                self.sweep_reload = false;
            } else if self.sweep_divider != 0 {
                self.sweep_divider -= 1;
            } else {
                self.period = self.sweep_timer();
                self.sweep_divider = self.sweep_load();
            }
        }

        self.forced_clock = false;
        if !self.duty() || self.length_counter == 0 || self.timer_counter < 8 {
            0
        } else {
            self.sweep_output()
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
