#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::apu::ApuSnapshot;
use crate::bus::{AddressBus, AndEqualsAndMask, DeviceKind};
use crate::channel::Channel;
use crate::cpu::dma::DmcDmaKind;
use crate::region::Region;

#[cfg_attr(feature = "save-states", derive(SaveState))]
#[derive(Default)]
pub struct Dmc {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
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
    regs: [u8; 4],
    dmc_req: Option<DmcDmaKind>,
}

impl Dmc {
    pub fn new(region: Region) -> Dmc {
        Dmc {
            region,
            ..Default::default()
        }
    }

    pub fn dmc_read(&mut self, value: u8) {
        self.read_pending = false;
        self.sample_buffer = value;
        self.sample_buffer_empty = false;
        self.address_counter = self.address_counter.wrapping_add(1);
        self.address_counter |= 0x8000;
        self.bytes_remaining = self.bytes_remaining.saturating_sub(1);
        if self.bytes_remaining == 0 {
            if self.loop_enabled() {
                self.bytes_remaining = self.sample_length();
                self.address_counter = self.sample_address();
            } else if self.irq_enabled() {
                self.irq = true;
            }
        }
    }

    pub fn get_irq(&self) -> bool {
        self.irq
    }

    pub fn get_dmc_req(&mut self) -> Option<DmcDmaKind> {
        self.dmc_req.take()
    }

    fn irq_enabled(&self) -> bool {
        self.regs[0] & 0x80 != 0
    }

    fn loop_enabled(&self) -> bool {
        self.regs[0] & 0x40 != 0
    }

    fn rate(&self) -> u16 {
        let rates = self.region.dmc_rates();
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

impl Channel for Dmc {
    fn register(&self, cpu: &mut AddressBus) {
        cpu.register_write(DeviceKind::Dmc, AndEqualsAndMask(0xfffc, 0x4010, 0x3));
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.regs[addr as usize] = value;
        match addr {
            0 => {
                if !self.irq_enabled() {
                    self.irq = false;
                }
            }
            1 => {
                self.output_value = self.direct_load();
            }
            2 => {}
            3 => {}
            _ => unreachable!(),
        }
    }

    fn tick(&mut self, _state: ApuSnapshot) -> u8 {
        self.current_tick += 1;

        if !self.read_pending && self.sample_buffer_empty && self.bytes_remaining != 0 {
            self.dmc_req = Some(DmcDmaKind::Reload(self.address_counter));
            self.read_pending = true;
        }

        if self.timer_counter != 0 {
            self.timer_counter -= 1
        } else {
            self.timer_counter = self.rate() - 1;
            if !self.silence {
                let offset = if self.output_shifter & 1 == 1 {
                    if self.output_value <= 125 {
                        2
                    } else {
                        0
                    }
                } else if self.output_value >= 2 {
                    -2
                } else {
                    0
                };
                self.output_value = ((self.output_value as i32) + offset) as u8;

                self.output_shifter >>= 1;
            }
            if self.bits_remaining != 0 {
                self.bits_remaining -= 1;
            }
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if self.sample_buffer_empty {
                    self.silence = true;
                } else {
                    self.silence = false;
                    self.output_shifter = self.sample_buffer;
                    self.sample_buffer_empty = true;
                }
            }
        }

        self.output_value
    }

    fn enable(&mut self) {
        if self.bytes_remaining == 0 {
            self.bytes_remaining = self.sample_length();
            self.address_counter = self.sample_address();
        }

        if !self.read_pending && self.sample_buffer_empty && self.bytes_remaining != 0 {
            self.read_pending = true;
            self.dmc_req = Some(DmcDmaKind::Load(self.address_counter));
        }

        self.irq = false;
    }

    fn disable(&mut self) {
        self.bytes_remaining = 0;
        self.irq = false;
    }

    fn get_state(&self) -> bool {
        self.bytes_remaining > 0
    }
}
