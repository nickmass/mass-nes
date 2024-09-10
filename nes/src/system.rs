use crate::apu::Apu;
use crate::bus::{Address, AddressBus, BusKind, DeviceKind, DeviceMappings, RangeAndMask};
use crate::cartridge::Cartridge;
use crate::cpu::{Cpu, CpuPinIn, TickResult};
use crate::debug::{Debug, DebugState};
use crate::input::Input;
use crate::mapper::Mapper;
use crate::memory::{MemoryBlock, Pages};
use crate::ppu::{Ppu, PpuState};

pub use crate::input::{Controller, InputDevice};

#[derive(Debug, Copy, Clone)]
pub enum UserInput {
    PlayerOne(Controller),
    Power,
    Reset,
}

#[derive(Debug, Copy, Clone)]
pub enum Region {
    Ntsc,
    Pal,
}

impl Default for Region {
    fn default() -> Self {
        Region::Ntsc
    }
}

impl Region {
    pub const fn frame_ticks(&self) -> f64 {
        match *self {
            Region::Ntsc => 29780.5,
            Region::Pal => 33247.5,
        }
    }

    pub const fn default_palette(&self) -> &'static [u8; 1536] {
        match *self {
            Region::Ntsc => include_bytes!("default.pal"),
            Region::Pal => include_bytes!("default.pal"),
        }
    }

    pub const fn vblank_line(&self) -> u32 {
        match *self {
            Region::Ntsc => 240,
            Region::Pal => 239,
        }
    }

    pub const fn prerender_line(&self) -> u32 {
        match *self {
            Region::Ntsc => 261,
            Region::Pal => 310,
        }
    }

    pub const fn uneven_frames(&self) -> bool {
        match *self {
            Region::Ntsc => true,
            Region::Pal => false,
        }
    }

    pub const fn emph_bits(&self) -> EmphMode {
        match *self {
            Region::Ntsc => EmphMode::Bgr,
            Region::Pal => EmphMode::Brg,
        }
    }

    pub const fn extra_ppu_tick(&self) -> bool {
        match *self {
            Region::Ntsc => false,
            Region::Pal => true,
        }
    }

    pub const fn refresh_rate(&self) -> f64 {
        match *self {
            Region::Ntsc => 60.0988,
            Region::Pal => 50.007,
        }
    }

    pub const fn five_step_seq(&self) -> &'static [u32] {
        match *self {
            Region::Ntsc => FIVE_STEP_SEQ_NTSC,
            Region::Pal => FIVE_STEP_SEQ_PAL,
        }
    }

    pub const fn four_step_seq(&self) -> &'static [u32] {
        match *self {
            Region::Ntsc => FOUR_STEP_SEQ_NTSC,
            Region::Pal => FOUR_STEP_SEQ_PAL,
        }
    }

    pub const fn dmc_rates(&self) -> &'static [u16] {
        match *self {
            Region::Ntsc => DMC_RATES_NTSC,
            Region::Pal => DMC_RATES_PAL,
        }
    }
}

const FIVE_STEP_SEQ_NTSC: &[u32] = &[7457, 14913, 22371, 37281, 37282];
const FIVE_STEP_SEQ_PAL: &[u32] = &[8314, 16628, 24940, 33254, 41566];

const FOUR_STEP_SEQ_NTSC: &[u32] = &[7457, 14913, 22371, 29829, 29830];
const FOUR_STEP_SEQ_PAL: &[u32] = &[8314, 16626, 24940, 33254, 33255];

const DMC_RATES_NTSC: &[u16] = &[
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];
const DMC_RATES_PAL: &[u16] = &[
    398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
];

#[derive(Debug, Copy, Clone)]
pub enum EmphMode {
    Bgr,
    Brg,
}

pub struct Machine {
    pub state: Box<SystemState>,
    pub system: System,
    region: Region,
    cycle: u64,
}

impl Machine {
    pub fn new(region: Region, cartridge: Cartridge) -> Machine {
        let mut state = Box::new(SystemState::default());
        let system = System::new(region, cartridge, &mut state);

        //system.debug.log_for(&mut state, 1000);
        //system.debug.log_range(&mut state, 0xe345, 0xe358);

        Machine {
            state,
            system,
            region,
            cycle: 0,
        }
    }

    pub fn force_power_up_pc(&mut self, addr: u16) {
        self.system.cpu.power_up_pc(Some(addr));
    }

    pub fn region(&self) -> Region {
        self.region
    }

    pub fn run(&mut self) {
        let last_frame = self.state.ppu.frame;
        //self.system.debug.log_for(&mut self.state, 1000);
        while self.state.ppu.frame == last_frame {
            let tick_result = self.system.cpu.tick(self.system.cpu_pin_in);

            let cpu_state = self.system.cpu.debug_state();
            let ppu_state = self.system.ppu.debug_state(&mut self.state);
            self.system
                .debug
                .trace(&self.system, &mut self.state, cpu_state, ppu_state);

            if self.state.debug.logging_range {
                eprintln!("{:2x?}   {}", tick_result, self.system.cpu_pin_in.data);
            }

            self.system.apu.tick();

            self.system.mapper.tick(&self.system, &mut self.state);

            let apu_irq = self.system.apu.get_irq();
            let mapper_irq = self.system.mapper.get_irq();

            match tick_result {
                TickResult::Read(addr) => {
                    let value = self
                        .system
                        .cpu_bus
                        .read(&self.system, &mut self.state, addr);
                    self.system.cpu_pin_in.data = value;
                }
                TickResult::Write(addr, value) => {
                    self.system
                        .cpu_bus
                        .write(&self.system, &mut self.state, addr, value)
                }
                // DMC Read holding bus
                TickResult::Idle => {}
                TickResult::DmcRead(value) => self.system.apu.dmc.dmc_read(value),
            }

            self.system.cpu_pin_in.irq = apu_irq | mapper_irq;
            self.system.cpu_pin_in.dmc_req = self.system.apu.get_dmc_req();
            self.system.cpu_pin_in.oam_req = self.system.apu.get_oam_req();

            self.system.ppu.tick(&self.system, &mut self.state);
            let ppu_state = self.system.ppu.debug_state(&mut self.state);
            self.system
                .debug
                .trace_ppu(&self.system, &mut self.state, cpu_state, ppu_state);
            self.system.ppu.tick(&self.system, &mut self.state);
            let ppu_state = self.system.ppu.debug_state(&mut self.state);
            self.system
                .debug
                .trace_ppu(&self.system, &mut self.state, cpu_state, ppu_state);
            self.system.ppu.tick(&self.system, &mut self.state);
            let ppu_state = self.system.ppu.debug_state(&mut self.state);
            self.system
                .debug
                .trace_ppu(&self.system, &mut self.state, cpu_state, ppu_state);
            if self.system.region.extra_ppu_tick() && self.cycle % 5 == 0 {
                self.system.ppu.tick(&self.system, &mut self.state);
            }

            self.system.cpu_pin_in.power = false;
            self.system.cpu_pin_in.reset = false;
            self.cycle += 1;
        }
    }

    pub fn get_screen(&mut self) -> &[u16] {
        &self.state.ppu.screen
    }

    pub fn get_audio(&mut self) -> &[i16] {
        self.system.apu.get_samples()
    }

    pub fn get_debug(&mut self) -> (&System, &mut SystemState) {
        (&self.system, &mut self.state)
    }

    pub fn set_input<T: IntoIterator<Item = UserInput>>(&mut self, input: T) {
        let input = input.into_iter();
        for i in input {
            self.handle_input(i);
        }
    }

    pub fn handle_input(&mut self, input: UserInput) {
        match input {
            UserInput::PlayerOne(c) => self.system.input.set_input(c.to_byte()),
            UserInput::Power => self.system.power(&mut self.state),
            UserInput::Reset => self.system.reset(&mut self.state),
        }
    }
}

#[derive(Default)]
pub struct SystemState {
    pub ppu: PpuState,
    pub mem: Pages,
    pub mappings: DeviceMappings,
    pub debug: DebugState,
}

pub struct System {
    pub region: Region,
    pub ppu: Ppu,
    pub cpu: Cpu,
    pub cpu_bus: AddressBus,
    pub cpu_mem: MemoryBlock,
    pub apu: Apu,
    pub cartridge: Cartridge,
    pub debug: Debug,
    pub input: Input,
    pub mapper: Box<dyn Mapper>,
    cpu_pin_in: CpuPinIn,
}

impl System {
    pub fn new(region: Region, cartridge: Cartridge, state: &mut SystemState) -> System {
        let cpu = Cpu::new();
        let ppu = Ppu::new(state, region);
        let apu = Apu::new(region);
        let cpu_bus = AddressBus::new(BusKind::Cpu, state, 0, 0xffff);
        let cpu_mem = MemoryBlock::new(2, &mut state.mem);
        let mapper = cartridge.get_mapper(state, &ppu);

        let mut system = System {
            region,
            ppu,
            cpu,
            cpu_bus,
            cpu_mem,
            apu,
            cartridge,
            debug: Debug::new(),
            input: Input::new(),
            mapper,
            cpu_pin_in: CpuPinIn::default(),
        };

        system.cpu_bus.register_read(
            state,
            DeviceKind::CpuRam,
            RangeAndMask(0x0000, 0x2000, 0x07ff),
        );
        system.cpu_bus.register_write(
            state,
            DeviceKind::CpuRam,
            RangeAndMask(0x0000, 0x2000, 0x07ff),
        );

        system
            .cpu_bus
            .register_read(state, DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x2007));
        system
            .cpu_bus
            .register_write(state, DeviceKind::Ppu, RangeAndMask(0x2000, 0x4000, 0x2007));

        system.ppu.register_read(
            state,
            DeviceKind::Nametables,
            RangeAndMask(0x2000, 0x4000, 0xfff),
        );
        system.ppu.register_write(
            state,
            DeviceKind::Nametables,
            RangeAndMask(0x2000, 0x4000, 0xfff),
        );

        system
            .cpu_bus
            .register_read(state, DeviceKind::Apu, Address(0x4015));
        system
            .cpu_bus
            .register_write(state, DeviceKind::Apu, Address(0x4014));
        system
            .cpu_bus
            .register_write(state, DeviceKind::Apu, Address(0x4015));
        system
            .cpu_bus
            .register_write(state, DeviceKind::Apu, Address(0x4017));

        system
            .cpu_bus
            .register_read(state, DeviceKind::Input, Address(0x4016));
        system
            .cpu_bus
            .register_read(state, DeviceKind::Input, Address(0x4017));
        system
            .cpu_bus
            .register_write(state, DeviceKind::Input, Address(0x4016));

        system.mapper.register(
            state,
            &mut system.cpu_bus,
            &mut system.ppu,
            &system.cartridge,
        );
        system.apu.register(state, &mut system.cpu_bus);
        system.power(state);
        system
    }

    pub fn power(&mut self, state: &mut SystemState) {
        self.cpu_pin_in.power = true;
        self.apu.power();
        self.ppu.power(self, state);
    }

    pub fn reset(&mut self, state: &mut SystemState) {
        self.cpu_pin_in.reset = true;
        self.apu.reset();
        self.ppu.power(self, state);
    }
}
