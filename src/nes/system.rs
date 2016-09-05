use nes::bus::{DeviceMappings, RangeAndMask, NotAndMask, Address, DeviceKind};
use nes::cpu::{Cpu, CpuState};
use nes::ppu::{Ppu, PpuState};
use nes::apu::{Apu, ApuState};
use nes::cartridge::Cartridge;
use nes::memory::Pages;
use nes::debug::{Debug, DebugState};
use nes::input::{Input, InputState};

pub use nes::input::{Controller, InputDevice};

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

pub struct Machine<FR, FA, FC, FI, I, FD> where 
    FR: FnMut(&[u16;256*240]), 
    FA: FnMut(&[i16]), 
    FC: FnMut() -> bool, 
    FI: FnMut() -> I,
    FD: FnMut(&System, &mut SystemState),
    I: InputDevice {
    
    pub state: Box<SystemState>,
    pub system: System,
    on_render: FR,
    on_audio: FA,
    on_closed: FC,
    on_input: FI,
    on_debug: FD,
}

impl<FR, FA, FC, FI, I, FD> Machine<FR, FA, FC, FI, I, FD> where 
    FR: FnMut(&[u16;256*240]),
    FA: FnMut(&[i16]),
    FC: FnMut() -> bool,
    FI: FnMut() -> I,
    FD: FnMut(&System, &mut SystemState),
    I: InputDevice {

    pub fn new(region: Region, cartridge: Cartridge, render: FR, audio: FA,
               closed: FC, input: FI, debug: FD) -> Machine<FR, FA, FC, FI, I, FD> {
        
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);
        Machine {
            state: state,
            system: system,
            on_render: render,
            on_audio: audio,
            on_closed: closed,
            on_input: input,
            on_debug: debug,
        }
    }

    pub fn run(&mut self) {
        self.system.cpu.power(&self.system, &mut self.state);
        let mut last_vblank = false;
        let mut cycle: u64 = 0;
        loop {
            self.system.cpu.tick(&self.system, &mut self.state);
            self.system.apu.tick(&self.system, &mut self.state);
            self.system.cartridge.mapper.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            self.system.ppu.tick(&self.system, &mut self.state);
            if self.system.region.extra_ppu_tick() && cycle % 5 == 0 {
                self.system.ppu.tick(&self.system, &mut self.state);
            }
            cycle += 1;
            if self.state.ppu.in_vblank && !last_vblank {
                (self.on_audio)(self.system.apu.get_samples(&self.system, &mut self.state));
                (self.on_render)(&self.state.ppu.screen);
                let input = (self.on_input)().to_byte();
                self.state.input.input = input;
                (self.on_debug)(&self.system, &mut self.state);
            }
            last_vblank = self.state.ppu.in_vblank;
            if (self.on_closed)() {
                break;
            }
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
        let ppu = Ppu::new(Region::Ntsc, state);
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
        system
    }
}
