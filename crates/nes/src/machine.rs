use std::rc::Rc;

#[cfg(feature = "save-states")]
use nes_traits::SaveState;

use crate::apu::{Apu, ChannelPlayback};
use crate::bus::{AddressBus, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::Cartridge;
use crate::cpu::{Cpu, CpuPinIn, TickResult};
use crate::debug::{Debug, DebugEvent};
use crate::input::Input;
use crate::mapper::{RcMapper, SaveWram};
use crate::memory::{FixedMemoryBlock, Memory};
use crate::ppu::{FrameEnd, Ppu};
use crate::region::Region;
use crate::run_until::RunUntil;

pub use crate::input::{Controller, InputDevice};

pub trait BreakpointHandler {
    fn breakpoint(&mut self, debug: &Debug) -> bool;
}

impl BreakpointHandler for () {
    fn breakpoint(&mut self, _debug: &Debug) -> bool {
        false
    }
}

impl<T: FnMut(&Debug) -> bool> BreakpointHandler for T {
    fn breakpoint(&mut self, debug: &Debug) -> bool {
        self(debug)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum RunResult {
    Breakpoint,
    Done,
}

#[derive(Debug, Copy, Clone)]
pub enum UserInput {
    PlayerOne(Controller),
    PlayerTwo(Controller),
    Mapper(MapperInput),
    Power,
    Reset,
}

#[derive(Debug, Copy, Clone)]
pub enum MapperInput {
    Fds(FdsInput),
}

#[derive(Debug, Copy, Clone)]
pub enum FdsInput {
    SetDisk(Option<usize>),
}

#[cfg_attr(feature = "save-states", derive(SaveState))]
pub struct Machine {
    #[cfg_attr(feature = "save-states", save(skip))]
    region: Region,
    tick: u64,
    step_start_tick: u64,
    cpu_tick: Option<TickResult>,

    #[cfg_attr(feature = "save-states", save(nested))]
    pub(crate) ppu: Ppu,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub(crate) cpu: Cpu,
    #[cfg_attr(feature = "save-states", save(skip))]
    pub(crate) cpu_bus: AddressBus,
    pub(crate) cpu_mem: FixedMemoryBlock<2>,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub(crate) apu: Apu,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub(crate) input: Input,
    #[cfg_attr(feature = "save-states", save(nested))]
    pub(crate) mapper: RcMapper,
    #[cfg_attr(feature = "save-states", save(skip))]
    debug: Rc<Debug>,
    cpu_pin_in: CpuPinIn,
}

impl Machine {
    pub fn new(region: Region, cartridge: Cartridge) -> Machine {
        let cpu = Cpu::new(region);
        let debug = Rc::new(Debug::new());
        let mut cpu_bus = AddressBus::new(0, 0xffff);
        let cpu_mem = FixedMemoryBlock::new();
        let input = Input::new();
        let mapper = cartridge.build_mapper(debug.clone());
        let apu = Apu::new(region, mapper.clone());
        let ppu = Ppu::new(region, mapper.clone(), debug.clone());

        cpu_bus.register_read(DeviceKind::CpuRam, RangeAndMask(0x0000, 0x2000, 0x07ff));
        cpu_bus.register_write(DeviceKind::CpuRam, RangeAndMask(0x0000, 0x2000, 0x07ff));

        mapper.register(&mut cpu_bus);
        ppu.register(&mut cpu_bus);
        apu.register(&mut cpu_bus);
        input.register(&mut cpu_bus);

        let mut machine = Machine {
            region,
            tick: 0,
            step_start_tick: 0,
            cpu_tick: None,
            ppu,
            cpu,
            cpu_bus,
            cpu_mem,
            apu,
            input,
            mapper,
            debug,
            cpu_pin_in: CpuPinIn::default(),
        };

        machine.power();
        machine
    }

    #[cfg(feature = "debugger")]
    pub fn with_trace_fn<
        F: FnMut(crate::cpu::CpuDebugState, crate::ppu::PpuDebugState) -> () + 'static,
    >(
        self,
        trace_fn: F,
    ) -> Self {
        self.debug.trace_fn(trace_fn);
        self
    }

    #[cfg(feature = "debugger")]
    pub fn with_debug_mem(&mut self, addr: u16, size_kb: u16) {
        self.debug.register(&mut self.cpu_bus, addr, size_kb);
    }

    pub fn force_power_up_pc(&mut self, addr: u16) {
        self.cpu.power_up_pc(Some(addr));
    }

    pub fn region(&self) -> Region {
        self.region
    }

    pub fn frame(&self) -> u32 {
        self.ppu.frame()
    }

    pub fn run_with_breakpoints<H: BreakpointHandler, U: RunUntil>(
        &mut self,
        frame_end: FrameEnd,
        until: U,
        break_handler: H,
    ) -> RunResult {
        self.do_run(frame_end, until, break_handler)
    }

    pub fn run(&mut self) {
        self.do_run(FrameEnd::ClearVblank, crate::run_until::Frames(1), ());
    }

    fn do_run<H: BreakpointHandler, U: RunUntil>(
        &mut self,
        frame_end: FrameEnd,
        mut until: U,
        mut break_handler: H,
    ) -> RunResult {
        #[cfg(feature = "debugger")]
        let _ = self.debug.take_interest_notification();
        self.step_start_tick = self.tick;
        while !until.done() {
            if self.tick % 3 == 0 {
                self.cpu_pin_in.irq = self.apu.get_irq() | self.mapper.get_irq();
                self.cpu_pin_in.nmi = self.ppu.nmi();
                self.cpu_tick = Some(self.cpu.tick(self.cpu_pin_in, &mut until));
                self.cpu_pin_in.power = false;
                self.cpu_pin_in.reset = false;

                let cpu_state = self.cpu.debug_state();
                self.debug.trace(&self, cpu_state);

                if let Some(sample) = self.cpu.dma.dmc_sample() {
                    self.apu.dmc.dmc_read(sample);
                }

                self.apu.tick(&mut until);
                self.mapper.tick();
                self.tick_ppu(frame_end, &mut until);
            } else {
                self.tick_ppu(frame_end, &mut until);
                if let Some(tick) = self.cpu_tick.take() {
                    match tick {
                        TickResult::Fetch(addr) => {
                            let value = self.read(addr);
                            self.debug.event_with_data(DebugEvent::CpuExec(addr), value);
                            self.debug.fetch(addr);
                            self.cpu_pin_in.data = value;
                            until.add_instruction();
                        }
                        TickResult::Read(addr) => {
                            let value = self.read(addr);
                            self.cpu_pin_in.data = value;
                        }
                        TickResult::Write(addr, value) => self.write(addr, value),
                        // Idle ticks while DMC/OAM DMA holds the bus, this is a simplification as
                        // the behavior depends on the register and the model of console.
                        // 200x registers will see multiple reads and no idle cycles as the are
                        // external to the CPU, while 400X registers (controllers) will see one
                        // read per contiguous set of writes so using idle cycles may be useful.
                        TickResult::Idle(addr) => {
                            if addr & 0xff00 != 0x4000 {
                                self.read(addr);
                            }
                        }
                    }

                    if let Some(addr) = self.apu.get_dmc_req() {
                        self.cpu.dma.request_dmc_dma(addr);
                    }

                    if let Some(addr) = self.apu.get_oam_req() {
                        self.cpu.dma.request_oam_dma(addr as u16);
                    }
                }
            }

            // Putting this tick here prevents single stepping to this specific PAL dot, not ideal
            if self.region().extra_ppu_tick() && self.tick % 15 == 0 {
                self.tick_ppu(frame_end, &mut until);
                self.step_start_tick = self.step_start_tick.saturating_sub(1);
            }

            self.tick += 1;

            if self.debug.breakpoint(&mut break_handler) {
                self.watch();
                return RunResult::Breakpoint;
            }
        }

        self.watch();
        RunResult::Done
    }

    fn tick_ppu<U: RunUntil>(&mut self, frame_end: FrameEnd, until: &mut U) {
        self.ppu.tick(frame_end, until);
        let ppu_state = self.ppu.debug_state();
        self.debug.trace_ppu(&self, ppu_state);
    }

    fn watch(&self) {
        #[cfg(feature = "debugger")]
        {
            let mut visitor = self.debug.watch_visitor();
            self.cpu.watch(&mut visitor);
            self.ppu.watch(&mut visitor);
            self.apu.watch(&mut visitor);
            self.mapper.watch(&mut visitor);
            let mut emu = visitor.group("Emulator");
            emu.value("Step Ticks", (self.tick - self.step_start_tick) as u32);
        }
    }

    fn read(&mut self, addr: u16) -> u8 {
        let value = match self.cpu_bus.read_addr(addr) {
            Some((addr, DeviceKind::CpuRam)) => self.cpu_mem.read(addr),
            Some((addr, DeviceKind::Ppu)) => self.ppu.read(addr),
            Some((addr, DeviceKind::Mapper)) => self.mapper.read(BusKind::Cpu, addr),
            Some((addr, DeviceKind::Input)) => self.input.read(addr, self.cpu_bus.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => self.apu.read(addr, self.cpu_bus.open_bus.get()),
            Some((addr, DeviceKind::Debug)) => self.debug.read(addr),
            None => self.cpu_bus.open_bus.get(),
            _ => unimplemented!(),
        };
        self.debug.event_with_data(DebugEvent::CpuRead(addr), value);

        if addr != 0x4015 {
            self.cpu_bus.open_bus.set(value);
        }

        value
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.debug
            .event_with_data(DebugEvent::CpuWrite(addr), value);
        use crate::channel::Channel;
        self.cpu_bus.open_bus.set(value);
        // Loop through potential mappings to allow MMC5 to snoop on PPU register writes
        for mapping in self.cpu_bus.write_addrs(addr) {
            match mapping {
                (addr, DeviceKind::CpuRam) => self.cpu_mem.write(addr, value),
                (addr, DeviceKind::Ppu) => self.ppu.write(addr, value),
                (addr, DeviceKind::Mapper) => self.mapper.write(BusKind::Cpu, addr, value),
                (addr, DeviceKind::Input) => self.input.write(addr, value),
                (addr, DeviceKind::Apu) => self.apu.write(addr, value),
                (addr, DeviceKind::PulseOne) => self.apu.pulse_one.write(addr, value),
                (addr, DeviceKind::PulseTwo) => self.apu.pulse_two.write(addr, value),
                (addr, DeviceKind::Noise) => self.apu.noise.write(addr, value),
                (addr, DeviceKind::Triangle) => self.apu.triangle.write(addr, value),
                (addr, DeviceKind::Dmc) => self.apu.dmc.write(addr, value),
                (addr, DeviceKind::Debug) => self.debug.write(addr, value),
            }
        }
    }

    #[cfg(feature = "debugger")]
    pub fn peek(&self, addr: u16) -> u8 {
        match self.cpu_bus.read_addr(addr) {
            Some((addr, DeviceKind::CpuRam)) => self.cpu_mem.read(addr),
            Some((addr, DeviceKind::Ppu)) => self.ppu.peek(addr),
            Some((addr, DeviceKind::Mapper)) => self.mapper.peek(BusKind::Cpu, addr),
            Some((addr, DeviceKind::Input)) => self.input.peek(addr, self.cpu_bus.open_bus.get()),
            Some((addr, DeviceKind::Apu)) => self.apu.peek(addr),
            Some((addr, DeviceKind::Debug)) => self.debug.read(addr),
            None => self.cpu_bus.open_bus.get(),
            _ => unimplemented!(),
        }
    }

    #[cfg(feature = "debugger")]
    pub fn peek_ppu(&self, addr: u16) -> u8 {
        self.ppu.ppu_peek(addr)
    }

    #[cfg(feature = "debugger")]
    pub fn set_debug_interest<I: Iterator<Item = DebugEvent>>(&self, iter: I) {
        self.debug.set_interest(iter);
    }

    #[cfg(feature = "debugger")]
    pub fn read_debug_events<F: FnMut(&[(u8, u16)])>(&self, reader: F) {
        self.debug.read_events(reader);
    }

    pub fn get_debug(&self) -> &Debug {
        &self.debug
    }

    pub fn get_screen(&self) -> &[u16] {
        self.ppu.screen()
    }

    pub fn get_samples(&mut self) -> impl Iterator<Item = i16> + '_ {
        self.apu.samples()
    }

    pub fn take_samples(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = i16> + ExactSizeIterator + '_ {
        self.apu.take_samples()
    }

    #[cfg(feature = "debugger")]
    pub fn take_channel_samples(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = crate::ChannelSamples> + ExactSizeIterator + '_ {
        self.apu.take_channel_samples()
    }

    pub fn set_channel_playback(&mut self, playback: ChannelPlayback) {
        self.apu.set_channel_playback(playback);
    }

    pub fn set_input<T: IntoIterator<Item = UserInput>>(&mut self, input: T) {
        let input = input.into_iter();
        for i in input {
            self.handle_input(i);
        }
    }

    pub fn handle_input(&mut self, input: UserInput) {
        match input {
            UserInput::PlayerOne(c) => self.input.set_port_one(c.to_byte()),
            UserInput::PlayerTwo(c) => self.input.set_port_two(c.to_byte()),
            UserInput::Power => self.power(),
            UserInput::Reset => self.reset(),
            UserInput::Mapper(mapper_input) => self.mapper.input(mapper_input),
        }
    }

    pub fn power(&mut self) {
        self.cpu_pin_in.power = true;
        for addr in 0u16..0x800 {
            let v = if addr & 1 == 1 { 0xff } else { 0x00 };
            self.cpu_mem.write(addr, v);
        }
        self.apu.power();
        self.ppu.power();
    }

    pub fn reset(&mut self) {
        self.cpu_pin_in.reset = true;
        self.apu.reset();
        self.ppu.reset();
    }

    pub fn save_wram(&self) -> Option<SaveWram> {
        self.mapper.save_wram()
    }

    #[cfg(feature = "save-states")]
    pub fn save_state(&self) -> crate::SaveData {
        crate::SaveData(<Self as SaveState>::save_state(self))
    }

    #[cfg(feature = "save-states")]
    pub fn restore_state(&mut self, state: &crate::SaveData) {
        <Self as SaveState>::restore_state(self, &state.0)
    }
}
