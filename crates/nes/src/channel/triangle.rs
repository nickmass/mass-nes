use crate::apu::ApuState;
use crate::bus::{AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::Channel;

use std::cell::RefCell;

#[derive(Default)]
struct TriangleState {
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

impl TriangleState {
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

pub struct Triangle {
    state: RefCell<TriangleState>,
}

impl Triangle {
    pub fn new() -> Triangle {
        let state = TriangleState {
            ..Default::default()
        };

        Triangle {
            state: RefCell::new(state),
        }
    }

    pub fn forced_clock(&self) {
        let mut channel = self.state.borrow_mut();
        channel.forced_clock = true;
    }
}

impl Channel for Triangle {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_write(DeviceKind::Triangle, AndEqualsAndMask(0xfffc, 0x4008, 0x3));
    }

    fn write(&self, addr: u16, value: u8) {
        let mut channel = self.state.borrow_mut();
        channel.regs[addr as usize] = value;
        match addr {
            0 => {}
            1 => {}
            2 => {}
            3 => {
                channel.length_counter = channel.length_load();
                channel.linear_reload = true;
            }
            _ => unreachable!(),
        }
    }

    fn tick(&self, state: &ApuState) -> u8 {
        let mut channel = self.state.borrow_mut();
        channel.current_tick += 1;

        if channel.timer_counter == 0 {
            channel.timer_counter = channel.timer_load();
            if channel.length_counter != 0 && channel.linear_counter != 0 {
                channel.sequencer = channel.sequencer.wrapping_add(1);
            }
        } else {
            channel.timer_counter -= 1;
        }

        if (state.is_quarter_frame() || channel.forced_clock) && channel.current_tick & 1 == 0 {
            if channel.linear_reload {
                channel.linear_counter = channel.linear_load();
            } else if channel.linear_counter != 0 {
                channel.linear_counter -= 1;
            }
            if !channel.halt() {
                channel.linear_reload = false;
            }
        }

        if (state.is_half_frame() || channel.forced_clock)
            && channel.length_counter != 0
            && !channel.halt()
        {
            channel.length_counter -= 1;
        }

        channel.forced_clock = false;

        channel.sequence()
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
