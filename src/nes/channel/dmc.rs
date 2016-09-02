use nes::bus::{DeviceKind, AndEqualsAndMask};
use nes::system::{System, SystemState};
use nes::cpu::Cpu;
use nes::channel::Channel;

use std::cell::RefCell;

#[derive(Default)]
struct DmcState {
    current_tick: u64,
    timer_counter: u16,
    sample_buffer: u8,
    sample_buffer_empty: bool,
    address_counter: u16,
    bytes_remaining: u16,
    output_value: u8,
    output_shifter: u8,
    bits_remaining: u8,
    read_pending: bool,
    irq: bool,
    silence: bool,
    regs: [u8;4],
}

impl DmcState {
    fn irq_enabled(&self) -> bool {
        self.regs[0] & 0x80 != 0
    }

    fn loop_enabled(&self) -> bool {
        self.regs[0] & 0x40 != 0
    }

    fn rate(&self) -> u16 {
        let rates = [428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128,
                    106, 84, 72, 54];
        rates[(self.regs[0] & 0xf) as usize]
    }

    fn direct_load(&self) -> u8 {
        self.regs[1] & 0x7f
    }

    fn sample_address(&self) -> u16 {
        ((self.regs[2] as u16) << 6) | 0xc000
    }

    fn sample_length(&self) -> u16 {
        ((self.regs[3] as u16) << 4) | 1
    }
}

pub struct Dmc {
    state: RefCell<DmcState>,
}

impl Dmc {
    pub fn new() -> Dmc {
        let state = DmcState {
            .. Default::default()
        };

        Dmc {
            state: RefCell::new(state),
        }
    }

    pub fn dmc_read(&self, value: u8) {
        let mut channel = self.state.borrow_mut();
        channel.read_pending = false;
        channel.sample_buffer = value;
        channel.sample_buffer_empty = false;
        channel.address_counter  = channel.address_counter.wrapping_add(1);
        channel.address_counter |= 0x8000;
        channel.bytes_remaining -= 1;
        if channel.bytes_remaining == 0 {
            if channel.loop_enabled() {
                channel.bytes_remaining = channel.sample_length();
                channel.address_counter = channel.sample_address();
            } else if channel.irq_enabled() {
                channel.irq = true;
            }
        }
    }

    pub fn get_irq(&self) -> bool {
        let channel = self.state.borrow();
        channel.irq
    }
}

impl Channel for Dmc {
    fn register(&self, state: &mut SystemState, cpu: &mut Cpu) {
        cpu.register_write(state, DeviceKind::Dmc, AndEqualsAndMask(0xfffc,
                                                                         0x4010, 0x3));
    }

    fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        0
    }

    fn write(&self, system: &System, state: &mut SystemState, addr: u16, value: u8) {
        let mut channel = self.state.borrow_mut();
        channel.regs[addr as usize] = value;
        match addr {
            0 => {
                if !channel.irq_enabled() { channel.irq = false; }
            },
            1 => {
                channel.output_value = channel.direct_load();
            },
            2 => {
            },
            3 => {
            },
            _ => unreachable!(),
        }
    }

    fn tick(&self, system: &System, state: &mut SystemState) -> u8 {
        let mut channel = self.state.borrow_mut();
        channel.current_tick += 1;

        if channel.irq {
            state.cpu.irq_req();
        }

        if !channel.read_pending && channel.sample_buffer_empty && 
            channel.bytes_remaining != 0 {
            state.cpu.dmc_req(channel.address_counter);
            channel.read_pending = true;
        }

        if channel.timer_counter != 0 {
            channel.timer_counter -= 1
        } else {
            channel.timer_counter = channel.rate();
            if !channel.silence {
                let offset = if channel.output_shifter & 1 == 1 {
                    if channel.output_value <= 125 { 2 } else { 0 }
                } else {
                    if channel.output_value >= 2 { -2 } else { 0 }
                };
                channel.output_value = ((channel.output_value as i32) + offset) as u8;

                channel.output_shifter >>= 1;
            }
            channel.bits_remaining -= 1;
            if channel.bits_remaining == 0 {
                channel.bits_remaining = 8;
                if channel.sample_buffer_empty {
                    channel.silence = true;
                } else {
                    channel.silence = false;
                    channel.output_shifter = channel.sample_buffer;
                    channel.sample_buffer_empty = true;
                }
            }
        }

        channel.output_value 
    }

    fn enable(&self, system: &System, state: &mut SystemState) {
        let mut channel = self.state.borrow_mut();
        if channel.bytes_remaining == 0 {
            channel.bytes_remaining = channel.sample_length();
            channel.address_counter = channel.sample_address();
        }
        channel.irq = false;
        
    }

    fn disable(&self, system: &System, state: &mut SystemState) {
        let mut channel = self.state.borrow_mut();
        channel.bytes_remaining = 0;
        channel.irq = false;
    }

    fn get_state(&self, system: &System, state: &mut SystemState) -> bool {
        let channel = self.state.borrow();
        channel.bytes_remaining > 0
    }
}
