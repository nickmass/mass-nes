use nes::bus::{DeviceKind, AndEqualsAndMask};
use nes::system::{System, SystemState};
use nes::cpu::Cpu;
use nes::channel::Channel;
use nes::apu;

use std::cell::RefCell;

#[derive(Copy, Clone)]
pub enum PulseChannel {
    InternalOne,
    InternalTwo,
}

impl Default for PulseChannel {
    fn default() -> PulseChannel { PulseChannel::InternalOne }
}

#[derive(Default)]
struct PulseState {
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
    regs: [u8;4],
    current_tick: u64,
    forced_clock: bool,
}

impl PulseState {
    fn timer_load(&self) -> u16 {
        (self.regs[2] as u16) | ((self.regs[3] as u16 & 7) << 8)
    }

    fn length_load(&self) -> u8 {
        if !self.enabled {
            0
        } else {
            apu::LENGTH_TABLE[(self.regs[3] >> 3 & 0x1f) as usize]
        }
    }

    fn envelope_volume(&self) -> u8 {
        self.regs[0] & 0xf
    }

    fn envelope_output(&self) -> u8 {
        if self.constant_volume() { self.envelope_volume() } else { self.decay_counter } 
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
                self.period - (period - chan)
        } else {
            self.period + period
        };

        period
    }

    fn sweep_timer(&self) -> u16 {
        let target = self.sweep_target();
        if self.period < 8 || target > 0x7ff || !self.sweep_enabled() || 
                self.shift_count() == 0 {
            self.period
        } else {
            target
        }
    }

    fn sweep_output(&self) -> u8 {
        let target = self.sweep_target();
        if self.period < 8 || target > 0x7ff { 0 } else { self.envelope_output() }
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

pub struct Pulse {
    state: RefCell<PulseState>,
    channel: PulseChannel,
}

impl Pulse {
    pub fn new(chan: PulseChannel) -> Pulse {
        let state = PulseState {
            channel: chan,
            .. Default::default()
        };

        Pulse {
            state: RefCell::new(state),
            channel: chan
        }
    }

    pub fn forced_clock(&self) {
        let mut channel = self.state.borrow_mut();
        channel.forced_clock = true;
    }
}

impl Channel for Pulse {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu) {
        match self.channel {
            PulseChannel::InternalOne => {
                cpu.register_write(state, DeviceKind::PulseOne, AndEqualsAndMask(0xfffc,
                                                                        0x4000, 0x3));
            },
            PulseChannel::InternalTwo => {
                cpu.register_write(state, DeviceKind::PulseTwo, AndEqualsAndMask(0xfffc,
                                                                        0x4004, 0x3));
            },
        }
    }

    fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        0
    }

    fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut channel = self.state.borrow_mut();
        channel.regs[addr as usize] = value;
        match addr {
            0 => {
            },
            1 => {
                channel.sweep_reload = true;
            },
            2 => {
                channel.period = channel.timer_load();
            },
            3 => {
                channel.period = channel.timer_load();
                channel.sequencer = 0;
                channel.length_counter = channel.length_load();
                channel.envelope_start = true;
            },
            _ => unreachable!(),
        }
    }

    fn tick(&self, system: &System, state: &mut SystemState) -> u8 {
        let mut channel = self.state.borrow_mut();
        channel.current_tick += 1;

        if channel.current_tick & 1 == 0 {
            if channel.timer_counter == 0 {
                channel.timer_counter = channel.period;
                channel.sequencer = channel.sequencer.wrapping_add(1);
            } else {
                channel.timer_counter -= 1;
            }
        }

        if state.apu.is_quarter_frame() || channel.forced_clock {
            if channel.envelope_start {
                channel.envelope_start = false;
                channel.decay_counter = 0xf;
                channel.envelope_divider = channel.envelope_volume();        
            } else {
                if channel.envelope_divider == 0 {
                    channel.envelope_divider = channel.envelope_volume();
                    if channel.decay_counter == 0 {
                        if channel.halt() { channel.decay_counter = 0xf }
                    } else {
                        channel.decay_counter -= 1;
                    }
                } else {
                    channel.envelope_divider -= 1;
                }
            }
        }

        if state.apu.is_half_frame() || channel.forced_clock {
            if channel.length_counter != 0 && !channel.halt() {
                channel.length_counter -= 1;
            }

            if channel.sweep_reload {
                if channel.sweep_divider == 0 {
                    channel.period = channel.sweep_timer();
                }
                channel.sweep_divider = channel.sweep_load();
                channel.sweep_reload = false;
            } else if channel.sweep_divider != 0 {
                channel.sweep_divider -= 1;
            } else {
                channel.period = channel.sweep_timer();
                channel.sweep_divider = channel.sweep_load();
            }
        }

        channel.forced_clock = false;
        if !channel.duty() || channel.length_counter == 0 || channel.timer_counter < 8 {
            0
        } else  {
            channel.sweep_output()
        }
    }

    fn enable(&self, system: &System, state: &mut SystemState) {
        let mut channel = self.state.borrow_mut();
        channel.enabled = true;
        
    }

    fn disable(&self, system: &System, state: &mut SystemState) {
        let mut channel = self.state.borrow_mut();
        channel.enabled = false;
        channel.length_counter = 0;
    }

    fn get_state(&self, system: &System, state: &mut SystemState) -> bool {
        let channel = self.state.borrow();
        channel.length_counter > 0
    }
}
