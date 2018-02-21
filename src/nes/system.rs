use bus::{DeviceMappings, RangeAndMask, NotAndMask, Address, DeviceKind};
use cpu::{Cpu, CpuState};
use ppu::{Ppu, PpuState};
use apu::{Apu, ApuState};
use cartridge::Cartridge;
use memory::Pages;
use debug::{Debug, DebugState};
use input::{Input, InputState};

pub use input::{Controller, InputDevice};

pub enum UserInput {
    PlayerOne(Controller),
    Power,
    Reset,
}

pub enum Region {
    Ntsc,
    Pal,
}
impl Region {
    pub fn frame_ticks(&self) -> f64 {
        match *self {
            Region::Ntsc => 29780.5,
            Region::Pal => 33247.5,
        }
    }

    pub fn default_palette(&self) -> &'static [u8; 1536] {
        match *self {
            Region::Ntsc => include_bytes!("default.pal"),
            Region::Pal => include_bytes!("default.pal"),
        }
    }

    pub fn vblank_line(&self) -> u32 {
        match *self {
            Region::Ntsc => 240,
            Region::Pal => 239,
        }
    }

    pub fn prerender_line(&self) -> u32 {
        match *self {
            Region::Ntsc => 261,
            Region::Pal => 310,
        }
    }

    pub fn uneven_frames(&self) -> bool {
        match *self {
            Region::Ntsc => true,
            Region::Pal => false,
        }
    }

    pub fn emph_bits(&self) -> EmphMode {
        match *self {
            Region::Ntsc => EmphMode::Bgr,
            Region::Pal => EmphMode::Brg,
        }
    }

    pub fn extra_ppu_tick(&self) -> bool {
        match *self {
            Region::Ntsc => false,
            Region::Pal => true,
        }
    }

    pub fn refresh_rate(&self) -> f64 {
        match *self {
            Region::Ntsc => 60.0988,
            Region::Pal => 50.007,
        }
    }

    pub fn five_step_seq(&self) -> &'static [u32] {
        match *self {
            Region::Ntsc => FIVE_STEP_SEQ_NTSC,
            Region::Pal => FIVE_STEP_SEQ_PAL,
        }
    }

    pub fn four_step_seq(&self) -> &'static [u32] {
        match *self {
            Region::Ntsc => FOUR_STEP_SEQ_NTSC,
            Region::Pal => FOUR_STEP_SEQ_PAL,
        }
    }

    pub fn dmc_rates(&self) -> &'static [u16] {
        match *self {
            Region::Ntsc => DMC_RATES_NTSC,
            Region::Pal => DMC_RATES_PAL,
        }
    }

}

const FIVE_STEP_SEQ_NTSC: &'static [u32] = &[7457, 14913, 22371, 37281, 37282];
const FIVE_STEP_SEQ_PAL: &'static [u32] = &[8314, 16628, 24940, 33254, 41566];

const FOUR_STEP_SEQ_NTSC: &'static [u32] = &[7457, 14913, 22371, 29829, 29830];
const FOUR_STEP_SEQ_PAL: &'static [u32] = &[8314, 16626, 24940, 33254, 33255];

const DMC_RATES_NTSC: &'static [u16] = &[428, 380, 340, 320, 286, 254, 226, 214,
                                        190, 160, 142, 128, 106, 84, 72, 54];
const DMC_RATES_PAL: &'static [u16] = &[398, 354, 316, 298, 276, 236, 210, 198,
                                        176, 148, 132, 118, 98, 78, 66, 50];


pub enum EmphMode {
    Bgr,
    Brg,
}

pub struct Machine {
    pub state: Box<SystemState>,
    pub system: System,
    cycle: u64,
}

impl Machine {
    pub fn new(region: Region, cartridge: Cartridge) -> Machine {
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);
        system.cpu.power(&system, &mut state);
        Machine {
            state: state,
            system: system,
            cycle: 0,
        }
    }

    pub fn run(&mut self) {
        let mut last_vblank = false;
        while self.state.ppu.in_vblank || !last_vblank {
            last_vblank = self.state.ppu.in_vblank;
            self.system.cpu.tick(&self.system, &mut self.state);
            self.system.apu.tick(&self.system, &mut self.state);
            self.system.cartridge.mapper.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            if self.system.region.extra_ppu_tick() && self.cycle % 5 == 0 {
                self.system.ppu.tick(&self.system, &mut self.state);
            }
            self.cycle += 1;
        }
    }

    pub fn get_screen(&mut self) -> &[u16] {
        &self.state.ppu.screen
    }

    pub fn get_audio(&mut self) -> &[i16] {
        self.system.apu.get_samples(&self.system, &mut self.state)
    }

    pub fn get_debug(&mut self) -> (&System, &mut SystemState) {
        (&self.system, &mut self.state)
    }

    pub fn set_input<T: IntoIterator<Item=UserInput>>(&mut self, input: T) {
        let input = input.into_iter();
        for i in input {
            self.handle_input(i);
        }
    }

    fn handle_input(&mut self, input: UserInput) {
        match input {
            UserInput::PlayerOne(c) => self.state.input.input = c.to_byte(),
            UserInput::Power => self.system.power(&mut self.state),
            UserInput::Reset => self.system.reset(&mut self.state),
        }
    }
}

#[derive(Default)]
pub struct SystemState {
    pub cpu: CpuState,
    pub ppu: PpuState,
    pub apu: ApuState,
    pub mem: Pages,
    pub mappings: DeviceMappings,
    pub input: InputState,
    pub debug: DebugState,
}

pub struct System {
    pub region: Region,
    pub ppu: Ppu,
    pub cpu: Cpu,
    pub apu: Apu,
    pub cartridge: Cartridge,
    pub debug: Debug,
    pub input: Input,
}

impl System {
    pub fn new(region: Region, mut cartridge: Cartridge,
               state: &mut SystemState) -> System {
        let cpu = Cpu::new(state);
        let ppu = Ppu::new(state);
        let apu = Apu::new(state);
        cartridge.init(state, &cpu, &ppu);

        let mut system = System {
            region: region,
            ppu: ppu,
            cpu: cpu,
            apu: apu,
            cartridge: cartridge,
            debug: Debug::new(),
            input: Input::new(),
        };

        system.cpu.register_read(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_write(state, DeviceKind::CpuRam,
                                 NotAndMask(0x7ff));
        system.cpu.register_read(state, DeviceKind::Ppu,
                                 RangeAndMask(0x2000, 0x4000, 0x2007));
        system.cpu.register_write(state, DeviceKind::Ppu,
                                  RangeAndMask(0x2000, 0x4000, 0x2007));
        system.cpu.register_write(state, DeviceKind::Ppu, Address(0x4014));
        system.ppu.register_read(state, DeviceKind::Nametables,
                                 RangeAndMask(0x2000, 0x4000, 0xfff));
        system.ppu.register_write(state, DeviceKind::Nametables,
                                 RangeAndMask(0x2000, 0x4000, 0xfff));
        system.cpu.register_read(state, DeviceKind::Apu, Address(0x4015));
        system.cpu.register_write(state, DeviceKind::Apu, Address(0x4015));
        system.cpu.register_write(state, DeviceKind::Apu, Address(0x4017));
        system.cpu.register_read(state, DeviceKind::Input, Address(0x4016));
        system.cpu.register_read(state, DeviceKind::Input, Address(0x4017));
        system.cpu.register_write(state, DeviceKind::Input, Address(0x4016));
        system.cartridge.mapper.register(state, &mut system.cpu, &mut system.ppu,
                               &system.cartridge);

        system.apu.register(state, &mut system.cpu);
        system.power(state);
        system
    }

    pub fn power(&self, state: &mut SystemState) {
        self.cpu.power(self, state);
        self.apu.power(self, state);
        self.ppu.power(self, state);
    }

    pub fn reset(&self, state: &mut SystemState) {
        self.cpu.reset(self, state);
        self.apu.reset(self, state);
        self.ppu.power(self, state);
    }
}
