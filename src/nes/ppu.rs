use std::rc::Rc;
use nes::bus::{AddressValidator, AddressBus, BusKind, DeviceKind, Address};
use nes::system::{Region, SystemState, System};
use nes::memory::MemoryBlock;

pub struct PpuState {
    current_tick: u64,
    regs: [u8;8],
    vblank: bool,
    sprite_zero_hit: bool,
    sprite_overflow: bool,
    last_write: u8,

    scroll_latch: bool,
    scroll_x: i32,
    scroll_y: i32,

    addr_latch: bool,
    data_addr: u16,
    data_read_buffer: u8,

    oam_addr: u8,
    oam_data: [u8; 256],

    current_frame: i32,
    current_line: i32,
    stage: Stage,

    palette_data: [u8; 32],
}

impl Default for PpuState {
    fn default() -> PpuState {
        PpuState {
            current_tick: Default::default(),
            regs: Default::default(),
            vblank: Default::default(),
            sprite_zero_hit: Default::default(),
            sprite_overflow: Default::default(),
            last_write: Default::default(),

            scroll_latch: Default::default(),
            scroll_x: Default::default(),
            scroll_y: Default::default(),

            addr_latch: Default::default(),
            data_addr: Default::default(),
            data_read_buffer: Default::default(),

            oam_addr: Default::default(),
            oam_data: [0; 256],

            current_frame: Default::default(),
            current_line: Default::default(),
            stage: Stage::Dot(0,0),

            palette_data: Default::default(),
        }
    }
}

impl PpuState {
    fn is_nmi_enabled(&self) -> bool { self.regs[0] & 0x80 != 0 }
    fn is_ext_bg(&self) -> bool { self.regs[0] & 0x40 != 0 }
    fn is_tall_sprites(&self) -> bool { self.regs[0] & 0x20 != 0 }

    fn background_pattern_table(&self) -> u16 {
        if self.regs[0] & 0x10 != 0 { 0x1000 } else { 0x0000 }
    }

    fn sprite_pattern_table(&self) -> u16 {
        if self.regs[0] & 0x08 != 0 { 0x1000 } else { 0x0000 }
    }

    fn vram_inc(&self) -> u16 {
        if self.regs[0] & 0x04 != 0 { 32 } else { 1 }
    }

    fn base_nametable(&self) -> u16 {
        match self.regs[0] & 3 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2c00,
            _ => unreachable!()
        }
    }

    fn is_blue_emph(&self) -> bool { self.regs[1] & 0x80 != 0 }
    fn is_green_emph(&self) -> bool { self.regs[1] & 0x40 != 0 }
    fn is_red_emph(&self) -> bool { self.regs[1] & 0x20 != 0 }
    fn is_sprites_enabled(&self) -> bool { self.regs[1] & 0x10 != 0 }
    fn is_background_enabled(&self) -> bool { self.regs[1] & 0x08 != 0 }
    fn is_left_sprites(&self) -> bool { self.regs[1] & 0x04 != 0 }
    fn is_left_baclground(&self) -> bool { self.regs[1] & 0x02 != 0 }
    fn is_grayscale(&self) -> bool { self.regs[1] & 0x01 != 0 }
    
    fn ppu_status(&self) -> u8 {
        let mut value = self.last_write & 0x1f;
        if self.sprite_overflow { value |= 0x20; }
        if self.sprite_zero_hit { value |= 0x40; }
        if self.vblank { value |= 0x80; }
        value
    }

    fn oam_address(&self) -> u8 { self.regs[3] }
}

enum Stage {
    Vblank(i32, u32),
    Hblank(i32, u32),
    Dot(i32, u32),
    Prerender(i32, u32),
}

impl Stage {
    fn increment(&self) -> Stage {
        match *self {
            Stage::Prerender(s, d) => {
                if d == 341 {
                    Stage::Dot(0, 0)
                } else {
                    Stage::Prerender(s, d + 1)
                }
            },
            Stage::Vblank(s, d) => {
                if d == 341 {
                    if s == 260 {
                        Stage::Prerender(261, 0)
                    } else {
                        Stage::Vblank(s + 1, 0)
                    }
                } else {
                    Stage::Vblank(s, d + 1)
                }
            },
            Stage::Hblank(s, d) => {
                if d == 341 {
                    if s == 239 {
                        Stage::Vblank(s + 1, 0)
                    } else {
                        Stage::Hblank(s + 1, 0)
                    }
                } else {
                    Stage::Hblank(s, d + 1)
                }
            },
            Stage::Dot(s, d) => {
                if d == 255 {
                    Stage::Hblank(s, d + 1)
                } else {
                    Stage::Dot(s, d + 1)
                }
            },
        }
    }
}

impl Default for Stage {
    fn default() -> Stage {
        Stage::Dot(0,0)
    }
}


pub struct Ppu {
    region: Region,
    pub mem: MemoryBlock,
    bus: AddressBus,
}

impl Ppu {
    pub fn new(region: Region, state: &mut SystemState) -> Ppu {
        let ppu = Ppu {
            region: region,
            bus: AddressBus::new(BusKind::Ppu, state, 0),
            mem: MemoryBlock::new(2, &mut state.mem),
        };

        ppu
    }

    pub fn register_read<T>(&mut self, state: &mut SystemState, device: DeviceKind, addr: T) where T: AddressValidator {
        self.bus.register_read(state, device, addr);
    }

    pub fn register_write<T>(&mut self, state: &mut SystemState, device: DeviceKind, addr: T) where T: AddressValidator {
        self.bus.register_write(state, device, addr);
    }

    pub fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16) -> u8 {
        match address {
            0 => state.ppu.last_write,
            1 => state.ppu.last_write,
            2 => {  
                state.ppu.addr_latch = false;
                state.ppu.scroll_latch = false;
                let status = state.ppu.ppu_status();
                state.ppu.vblank = false;
                status
            }, //PPUSTATUS
            3 => state.ppu.oam_addr, //OAMADDR
            4 => state.ppu.oam_data[state.ppu.oam_addr as usize], //OANDATA
            5 => state.ppu.last_write,
            6 => state.ppu.last_write,
            7 => { //PPUDATA
                let addr = state.ppu.data_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    state.ppu.palette_data[(addr & 0x1f) as usize]
                } else {
                    state.ppu.data_read_buffer
                };
                state.ppu.data_read_buffer = self.bus.read(system, state, addr);
                result
            }
            4014 => 0,
            _ => unreachable!(),
        }
    }

    pub fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16, value: u8) {
        match address {
            0 => state.ppu.regs[0] = value, //PPUCTRL
            1 => state.ppu.regs[1] = value, //PPUMASK
            2 => state.ppu.regs[2] = value,
            3 => state.ppu.oam_addr = value, //OAMADDR
            4 => state.ppu.oam_data[state.ppu.oam_addr as usize] = value, //OAMDATA
            5 => { //PPUSCROLL
                if state.ppu.scroll_latch {
                    state.ppu.scroll_y = value as i32; 
                } else {
                    state.ppu.scroll_x = value as i32;
                }
                state.ppu.scroll_latch = !state.ppu.scroll_latch;

            },
            6 => { //PPUADDR
                if state.ppu.addr_latch {
                    state.ppu.data_addr &= 0xff00;
                    state.ppu.data_addr |= value as u16
                } else {
                    state.ppu.data_addr &= 0x00ff;
                    state.ppu.data_addr |= (value as u16) << 8;
                }
                state.ppu.addr_latch = !state.ppu.addr_latch;
            },
            7 => { //PPUDATA
                let addr = state.ppu.data_addr;
                self.bus.write(system, state, addr, value);
                if addr & 0x3f00 == 0x3f00 {
                    state.ppu.palette_data[(addr & 0x1f) as usize] = value;
                }
                state.ppu.data_addr += state.ppu.vram_inc();
            },
            4014 => { //OAMDMA
                state.cpu.oam_dma_req(value);
            },
            _ => unreachable!(),
        }

        if address < 8 { state.ppu.last_write = value; }
    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.ppu.current_tick += 1;
        match state.ppu.stage {
            Stage::Vblank(241,1) => {
                state.ppu.vblank = true;
                if state.ppu.is_nmi_enabled() {
                    state.cpu.nmi_req();
                }
            },
            Stage::Prerender(261, 1) => {
                state.ppu.vblank = false;
            },
            _ => {}
        }
        state.ppu.stage = state.ppu.stage.increment();
    }
}
