use nes::bus::{DeviceKind, AndEqualsAndMask};
use nes::system::{System, SystemState};
use nes::cpu::Cpu;
use nes::channel::Channel;
use nes::apu;

use std::cell::RefCell;

#[derive(Default)]
struct NoiseState {
    timer_counter: u16,
    length_counter: u8,
    enabled: bool,
    shifter: u16,
    envelope_start: bool,
    envelope_divider: u8,
    decay_counter: u8,
    regs: [u8;4],
    current_tick: u64,
    forced_clock: bool,
}

impl NoiseState {
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
        self.shifter |= feedback  << 14;
    }

    fn timer_period(&self) -> u16 {
        let rates = [4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016,
                    2034, 4068]; //Im not sure if these are in CPU clocks, or APU clocks.
        rates[(self.regs[2] & 0xf) as usize]
    }
}

pub struct Noise {
    state: RefCell<NoiseState>,
}

impl Noise {
    pub fn new() -> Noise {
        let state = NoiseState {
            shifter: 1,
            .. Default::default()
        };

        Noise {
            state: RefCell::new(state),
        }
    }

    pub fn forced_clock(&self) {
        let mut channel = self.state.borrow_mut();
        channel.forced_clock = true;
    }
}

impl Channel for Noise {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu) {
        cpu.register_write(state, DeviceKind::Noise, AndEqualsAndMask(0xfffc,
                                                                         0x400c, 0x3));
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
            },
            2 => {
            },
            3 => {
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
                channel.timer_counter = channel.timer_period();
                channel.clock_shifter();             
            } else {
                channel.timer_counter -= 1;
            }
        }

        if state.apu.is_quarter_frame(system) || channel.forced_clock { 
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

        if state.apu.is_half_frame(system) || channel.forced_clock {
            if channel.length_counter != 0 && !channel.halt() {
                channel.length_counter -= 1;
            }
        }

        channel.forced_clock = false;

        if (channel.shifter & 1) == 1 || channel.length_counter == 0 {
            0
        } else  {
            channel.envelope_output()
        }
    }

    fn enable(&self) {
        let mut channel = self.state.borrow_mut();
        channel.enabled = true;
        
    }

    fn disable(&self) {
        let mut channel = self.state.borrow_mut();
        channel.enabled = false;
        channel.length_counter = 0;
    }

    fn get_state(&self) -> bool {
        let channel = self.state.borrow();
        channel.length_counter > 0
    }
}
