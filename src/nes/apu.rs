use nes::system::{System, SystemState};
use nes::channel::{Channel, Pulse, PulseChannel, Triangle, Noise, Dmc};
use nes::cpu::Cpu;

pub const LENGTH_TABLE: [u8; 0x20] = [10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14,
                                  12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22, 192,
                                  24, 72, 26, 16, 28, 32, 30];

pub struct ApuState {
    frame_counter: u32,
    five_step_mode: bool,
    irq_inhibit: bool,
    irq: bool,
    pub samples: [u8; 29781],
    sample_index: usize,
}

impl Default for ApuState {
    fn default() -> ApuState {
        ApuState {
            frame_counter: 0,
            five_step_mode: false,
            irq_inhibit: false,
            irq: false,
            samples: [0; 29781],
            sample_index: 0,
        }
    }
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
    pub pulse_one: Pulse,
    pub pulse_two: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
    pub dmc: Dmc,
}

impl Apu {
    pub fn new(state: &mut SystemState) -> Apu {
        Apu {
            pulse_one: Pulse::new(PulseChannel::InternalOne),
            pulse_two: Pulse::new(PulseChannel::InternalTwo),
            triangle: Triangle::new(),
            noise: Noise::new(),
            dmc: Dmc::new(),
        }
    }

    pub fn peek(&self, system: &System, state: &SystemState, addr: u16) -> u8 {
        0
    }

    pub fn read(&self, system: &System, state: &mut SystemState, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut val = 0;
                if self.pulse_one.get_state(system, state) { val |= 0x01; }
                if self.pulse_two.get_state(system, state) { val |= 0x02; }
                if self.triangle.get_state(system, state) { val |= 0x04; }
                if self.noise.get_state(system, state) { val |= 0x08; }
                if self.dmc.get_state(system, state) { val |= 0x10; }
                if state.apu.irq { val |= 0x40; }
                if self.dmc.get_irq() { val |= 0x80; }
                state.apu.irq = false;
                val
            },
            _ => unreachable!()
        }
    }

    pub fn write(&self, system: &System, state: &mut SystemState, addr: u16,
                 value: u8) {
        match addr {
            0x4015 => {
                if value & 1 != 0 {
                    self.pulse_one.enable(system, state);
                } else {
                    self.pulse_one.disable(system, state);
                }
                if value & 0x2 != 0 {
                    self.pulse_two.enable(system, state);
                } else {
                    self.pulse_two.disable(system, state);
                }
                if value & 0x4 != 0 {
                    self.triangle.enable(system, state);
                } else {
                    self.triangle.disable(system, state);
                }
                if value & 0x8 != 0 {
                    self.noise.enable(system, state);
                } else {
                    self.noise.disable(system, state);
                }
                if value & 0x10 != 0 {
                    self.dmc.enable(system, state);
                } else {
                    self.dmc.disable(system, state);
                }
            },
            0x4017 => {
                state.apu.five_step_mode = value & 0x80 != 0;
                state.apu.irq_inhibit = value & 0x40 != 0;
                if state.apu.irq_inhibit {
                    state.apu.irq = false
                }
                if state.apu.five_step_mode {
                    self.forced_clock();
                }
            },
            _ => unreachable!(),
        }
    }

    fn forced_clock(&self) {
        self.pulse_one.forced_clock();
        self.pulse_two.forced_clock();
        self.triangle.forced_clock();
        self.noise.forced_clock();
    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.apu.increment_frame_counter();
        if !state.apu.five_step_mode && !state.apu.irq_inhibit {
            if state.apu.frame_counter == 0 || state.apu.frame_counter == 29828 ||
                    state.apu.frame_counter == 29829 {
                state.apu.irq = true;
            }
        }
        if state.apu.irq {
            state.cpu.irq_req();
        }

        let pulse1 = self.pulse_one.tick(system, state);
        let pulse2 = self.pulse_two.tick(system, state);
        let triangle = self.triangle.tick(system, state);
        let noise = self.noise.tick(system, state);
        let dmc = self.dmc.tick(system, state);

        state.apu.samples[state.apu.sample_index] = (pulse1 + pulse2 + triangle + noise + dmc); 
        state.apu.sample_index += 1;
    }
    
    pub fn get_samples<'a>(&'a self, system: &'a System,
                           state: &'a mut SystemState) -> &[u8] {
        let index = state.apu.sample_index;
        state.apu.sample_index = 0;
        &state.apu.samples[0..index]
    }
    
    pub fn register(&self, state: &mut SystemState, cpu: &mut Cpu) {
        self.pulse_one.register(state, cpu);
        self.pulse_two.register(state, cpu);
        self.triangle.register(state, cpu);
        self.noise.register(state, cpu);
        self.dmc.register(state, cpu);
    }
}
