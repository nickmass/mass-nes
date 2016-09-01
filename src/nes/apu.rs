use nes::system::{System, SystemState};

pub const LENGTH_TABLE: [u8; 0x20] = [10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14,
                                  12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22, 192,
                                  24, 72, 26, 16, 28, 32, 30];

#[derive(Default)]
pub struct ApuState {
    frame_counter: u32,
    five_step_mode: bool,
}

impl ApuState {
    pub fn is_quarter_frame(&self) -> bool {
        if self.five_step_mode {
            self.frame_counter == 7457 ||
                self.frame_counter == 14913||
                self.frame_counter == 22371||
                self.frame_counter == 37281
        } else {
            self.frame_counter == 7457 ||
                self.frame_counter == 14913||
                self.frame_counter == 22371||
                self.frame_counter == 29829
        }
    }

    pub fn is_half_frame(&self) -> bool {
        if self.five_step_mode {
            self.frame_counter == 14913||
                self.frame_counter == 37281
        } else {
            self.frame_counter == 14913||
                self.frame_counter == 29829
        }
    }

    fn increment_frame_counter(&mut self) {
        if self.five_step_mode {
            if self.frame_counter >= 37282 {
                self.frame_counter = 0;
            } else {
                self.frame_counter += 1;
            }
        } else {
            if self.frame_counter >= 29830 {
                self.frame_counter = 0;
            } else {
                self.frame_counter += 1;
            }
        }
    }
}

pub struct Apu {
}

impl Apu {
    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.apu.increment_frame_counter();
        if state.apu.frame_counter == 0  {
            state.cpu.irq_req();
        }
    }

}
