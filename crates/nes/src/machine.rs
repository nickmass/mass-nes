use crate::apu::Apu;
use crate::bus::{AddressBus, BusKind, DeviceKind, RangeAndMask};
use crate::cartridge::Cartridge;
use crate::cpu::{Cpu, CpuPinIn, TickResult};
use crate::debug::Debug;
use crate::input::Input;
use crate::mapper::Mapper;
use crate::memory::MemoryBlock;
use crate::ppu::Ppu;
use crate::region::Region;

pub use crate::input::{Controller, InputDevice};

use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, Copy, Clone)]
pub enum UserInput {
    PlayerOne(Controller),
    Power,
    Reset,
}

pub struct Machine {
    region: Region,
    cycle: u64,

    pub(crate) ppu: Ppu,
    pub(crate) cpu: Cpu,
    pub(crate) cpu_bus: AddressBus,
    pub(crate) cpu_mem: MemoryBlock,
    pub(crate) apu: Apu,
    pub(crate) input: Input,
    pub(crate) mapper: Rc<dyn Mapper>,
    debug: Debug,
    cpu_pin_in: CpuPinIn,
}

impl Machine {
    pub fn new(region: Region, cartridge: Cartridge) -> Machine {
        let cpu = Cpu::new();
        let mut cpu_bus = AddressBus::new(BusKind::Cpu, 0, 0xffff);
        let cpu_mem = MemoryBlock::new(2);
        let apu = Apu::new(region);
        let input = Input::new();
        let mapper = cartridge.build_mapper();
        let ppu = Ppu::new(region, cpu.nmi.clone(), mapper.clone());

        cpu_bus.register_read(DeviceKind::CpuRam, RangeAndMask(0x0000, 0x2000, 0x07ff));
        cpu_bus.register_write(DeviceKind::CpuRam, RangeAndMask(0x0000, 0x2000, 0x07ff));

        mapper.register(&mut cpu_bus);
        ppu.register(&mut cpu_bus);
        apu.register(&mut cpu_bus);
        input.register(&mut cpu_bus);

        let mut machine = Machine {
            region,
            cycle: 0,
            ppu,
            cpu,
            cpu_bus,
            cpu_mem,
            apu,
            input,
            mapper,
            debug: Debug::new(),
            cpu_pin_in: CpuPinIn::default(),
        };

        machine.power();
        machine
    }

    pub fn with_trace_fn<
        F: FnMut(crate::cpu::CpuDebugState, crate::ppu::PpuDebugState) -> () + 'static,
    >(
        self,
        trace_fn: F,
    ) -> Self {
        self.debug.trace_fn(trace_fn);
        self
    }

    pub fn force_power_up_pc(&mut self, addr: u16) {
        self.cpu.power_up_pc(Some(addr));
    }

    pub fn region(&self) -> Region {
        self.region
    }

    #[tracing::instrument(skip_all)]
    pub fn run(&mut self) {
        let last_frame = self.ppu.frame();
        while self.ppu.frame() == last_frame {
            let tick_result = self.cpu.tick(self.cpu_pin_in);

            let cpu_state = self.cpu.debug_state();
            let ppu_state = self.ppu.debug_state();
            self.debug.trace(&self, cpu_state, ppu_state);

            self.apu.tick();
            self.mapper.tick();

            let apu_irq = self.apu.get_irq();
            let mapper_irq = self.mapper.get_irq();

            match tick_result {
                TickResult::Read(addr) => {
                    let value = self.cpu_bus.read(&self, addr);
                    self.cpu_pin_in.data = value;
                }
                TickResult::Write(addr, value) => self.cpu_bus.write(&self, addr, value),
                // DMC Read holding bus
                TickResult::Idle => {}
                TickResult::DmcRead(value) => self.apu.dmc.dmc_read(value),
            }

            self.cpu_pin_in.irq = apu_irq | mapper_irq;
            self.cpu_pin_in.dmc_req = self.apu.get_dmc_req();
            self.cpu_pin_in.oam_req = self.apu.get_oam_req();

            for _ in 0..3 {
                self.ppu.tick();
                let ppu_state = self.ppu.debug_state();
                self.debug.trace_ppu(&self, cpu_state, ppu_state);
            }

            if self.region.extra_ppu_tick() && self.cycle % 5 == 0 {
                self.ppu.tick();
                let ppu_state = self.ppu.debug_state();
                self.debug.trace_ppu(&self, cpu_state, ppu_state);
            }

            self.cpu_pin_in.power = false;
            self.cpu_pin_in.reset = false;
            self.cycle += 1;
        }
    }

    pub fn get_debug(&self) -> &Debug {
        &self.debug
    }

    pub fn get_screen(&mut self) -> &[Cell<u16>] {
        self.ppu.screen()
    }

    pub fn get_audio(&mut self) -> &[i16] {
        self.apu.get_samples()
    }

    pub fn set_input<T: IntoIterator<Item = UserInput>>(&mut self, input: T) {
        let input = input.into_iter();
        for i in input {
            self.handle_input(i);
        }
    }

    pub fn handle_input(&mut self, input: UserInput) {
        match input {
            UserInput::PlayerOne(c) => self.input.set_input(c.to_byte()),
            UserInput::Power => self.power(),
            UserInput::Reset => self.reset(),
        }
    }

    pub fn power(&mut self) {
        self.cpu_pin_in.power = true;
        self.apu.power();
        self.ppu.power();
    }

    pub fn reset(&mut self) {
        self.cpu_pin_in.reset = true;
        self.apu.reset();
        self.ppu.reset();
    }
}
