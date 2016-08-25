use std::rc::Rc;
use nes::bus::{AddressValidator, AddressBus, BusKind, DeviceKind, Address};
use nes::system::{Region, SystemState, System};
use nes::memory::MemoryBlock;

pub struct PpuState {
    current_tick: u64,
    regs: [u8;8],
    pub vblank: bool,
    sprite_zero_hit: bool,
    sprite_overflow: bool,
    last_write: u8,

    write_latch: bool,
    
    data_read_buffer: u8,

    vram_addr: u16,
    vram_addr_temp: u16,
    vram_fine_x: u16,

    oam_addr: u8,
    oam_data: [u8; 256],

    current_frame: i32,
    current_line: i32,
    stage: Stage,

    palette_data: [u8; 32],

    nametable_tile: u8,

    attribute_low: u8,
    attribute_high: u8,

    pattern_low: u8,
    pattern_high: u8,

    low_bg_shift: u16,
    high_bg_shift: u16,

    low_attr_shift: u8,
    high_attr_shift: u8,

    pub screen: [u8;256*240],
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

            write_latch: Default::default(),
            data_read_buffer: Default::default(),

            vram_addr: Default::default(),
            vram_addr_temp: Default::default(),
            vram_fine_x: Default::default(),

            oam_addr: Default::default(),
            oam_data: [0; 256],

            current_frame: Default::default(),
            current_line: Default::default(),
            stage: Stage::Dot(0,0),

            palette_data: Default::default(),

            nametable_tile: 0,
           
            attribute_low: 0,
            attribute_high: 0,

            pattern_low: 0,
            pattern_high: 0,

            low_bg_shift: 0,
            high_bg_shift: 0,

            low_attr_shift: 0,
            high_attr_shift: 0,

            screen: [0;256*240],
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
            0 => 0x000,
            1 => 0x400,
            2 => 0x800,
            3 => 0xc00,
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
    Vblank(u32, u32),
    Hblank(u32, u32),
    Dot(u32, u32),
    Prerender(u32, u32),
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
                        Stage::Dot(s + 1, 0)
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

    pub fn peek(&self, bus: BusKind, system: &System, state: &SystemState, address: u16) -> u8 {
        match address {
            0 => state.ppu.last_write,
            1 => state.ppu.last_write,
            2 => state.ppu.ppu_status(),
            3 => state.ppu.oam_addr, //OAMADDR
            4 => state.ppu.oam_data[state.ppu.oam_addr as usize], //OANDATA
            5 => state.ppu.last_write,
            6 => state.ppu.last_write,
            7 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    state.ppu.palette_data[(addr & 0x1f) as usize]
                } else {
                    state.ppu.data_read_buffer
                };
                result
            }
            4014 => 0,
            _ => unreachable!(),
        }
    }
    pub fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16) -> u8 {
        match address {
            0 => state.ppu.last_write,
            1 => state.ppu.last_write,
            2 => {  
                state.ppu.write_latch = false;
                let status = state.ppu.ppu_status();
                state.ppu.vblank = false;
                status
            }, //PPUSTATUS
            3 => state.ppu.oam_addr, //OAMADDR
            4 => state.ppu.oam_data[state.ppu.oam_addr as usize], //OANDATA
            5 => state.ppu.last_write,
            6 => state.ppu.last_write,
            7 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    state.ppu.palette_data[(addr & 0x1f) as usize]
                } else {
                    state.ppu.data_read_buffer
                };
                state.ppu.data_read_buffer = self.bus.read(system, state, addr);
                state.ppu.vram_addr += state.ppu.vram_inc();
                result
            }
            4014 => 0,
            _ => unreachable!(),
        }
    }

    pub fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16, value: u8) {
        match address {
            0 => {
                state.ppu.regs[0] = value;
                state.ppu.vram_addr_temp &= 0xc00;
                state.ppu.vram_addr_temp |= state.ppu.base_nametable();
            }, //PPUCTRL
            1 => state.ppu.regs[1] = value, //PPUMASK
            2 => state.ppu.regs[2] = value,
            3 => state.ppu.oam_addr = value, //OAMADDR
            4 => state.ppu.oam_data[state.ppu.oam_addr as usize] = value, //OAMDATA
            5 => { //PPUSCROLL
                if state.ppu.write_latch {
                    let value = value as u16;
                    state.ppu.vram_addr_temp &= 0xfc1f;
                    state.ppu.vram_addr_temp |= (value & 0xff07) << 2;
                    state.ppu.vram_addr_temp &= 0x0fff;
                    state.ppu.vram_addr_temp |= (value & 0x07) << 12;
                } else {
                    state.ppu.vram_addr_temp &= 0xffe0;
                    state.ppu.vram_addr_temp |= (value >> 3) as u16;
                    state.ppu.vram_fine_x = (value & 0x07) as u16;
                }
                state.ppu.write_latch = !state.ppu.write_latch;

            },
            6 => { //PPUADDR
                if state.ppu.write_latch {
                    state.ppu.vram_addr_temp &= 0xff00;
                    state.ppu.vram_addr_temp |= value as u16;
                    state.ppu.vram_addr = state.ppu.vram_addr_temp;
                } else {
                    state.ppu.vram_addr_temp &= 0x00ff;
                    state.ppu.vram_addr_temp |= ((value & 0x7f) as u16) << 8;
                }
                state.ppu.write_latch = !state.ppu.write_latch;
            },
            7 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                self.bus.write(system, state, addr, value);
                if addr & 0x3f00 == 0x3f00 {
                    state.ppu.palette_data[(addr & 0x1f) as usize] = value;
                }
                state.ppu.vram_addr += state.ppu.vram_inc();
            },
            0x4014 => { //OAMDMA
                state.cpu.oam_dma_req(value);
            },
            _ => {
                println!("{:4X} Address", address);
                unreachable!()
            },
        }

        if address < 8 { state.ppu.last_write = value; }
    }

    pub fn tick(&self, system: &System, state: &mut SystemState) {
        state.ppu.current_tick += 1;
        match state.ppu.stage {
            Stage::Prerender(1, 261) => {
                state.ppu.vblank = false;
                self.fetch_nametable(system, state);
            },
            Stage::Prerender(s, c) if c % 8 == 1 && c < 256 => {
                self.fetch_nametable(system, state);
            },
            Stage::Prerender(s, c) if c % 8 == 3 && c < 256 => {
                self.fetch_attribute(system, state);
            },
            Stage::Prerender(s, c) if c % 8 == 5 && c < 256 => {
                self.fetch_low_bg_pattern(system, state);
            },
            Stage::Prerender(s, c) if c % 8 == 7 && c < 256 => {
                self.fetch_high_bg_pattern(system, state);
            },
            Stage::Prerender(s, c) if c % 8 == 0 && c != 0 && c < 256 => {
                self.horz_increment(state);
                self.load_bg_shifters(state);
            },
            Stage::Prerender(s, c) if c >= 280 && c <= 304 => {
                self.vert_reset(state);
            },
            Stage::Prerender(s, c) if c  == 321 || c == 329 || c == 337 || c == 339 => {
                self.fetch_nametable(system, state);
            },
            Stage::Prerender(s, c) if c == 323 || c == 331  => {
                self.fetch_attribute(system, state);
            },
            Stage::Prerender(s, c) if c == 325 || c == 333 => {
                self.fetch_low_bg_pattern(system, state);
            },
            Stage::Prerender(s, c) if c == 327 || c == 335 => {
                self.fetch_high_bg_pattern(system, state);
            },
            Stage::Prerender(s, c) if c == 328 || c == 336 => {
                self.horz_increment(state);
                self.load_bg_shifters(state);
            },
            Stage::Dot(s, c) if c % 8 == 1 => {
                self.fetch_nametable(system, state);
            },
            Stage::Dot(s, c) if c % 8 == 3 => {
                self.fetch_attribute(system, state);
            },
            Stage::Dot(s, c) if c % 8 == 5 => {
                self.fetch_low_bg_pattern(system, state);
            },
            Stage::Dot(s, c) if c % 8 == 7 => {
                self.fetch_high_bg_pattern(system, state);
            },
            Stage::Dot(s, c) if c % 8 == 0 && c != 0 => {
                self.horz_increment(state);
                self.load_bg_shifters(state);
            },
            Stage::Hblank(s, c) if c  == 321 || c == 329 || c == 337 || c == 339 => {
                self.fetch_nametable(system, state);
            },
            Stage::Hblank(s, c) if c == 323 || c == 331  => {
                self.fetch_attribute(system, state);
            },
            Stage::Hblank(s, c) if c == 325 || c == 333 => {
                self.fetch_low_bg_pattern(system, state);
            },
            Stage::Hblank(s, c) if c == 327 || c == 335 => {
                self.fetch_high_bg_pattern(system, state);
            },
            Stage::Hblank(s, c) if c == 328 || c == 336 => {
                self.horz_increment(state);
                self.load_bg_shifters(state);
            },
            Stage::Hblank(s, 256) => {
                self.vert_increment(state);
            },
            Stage::Hblank(s, 257) => {
                self.horz_reset(state);
            },
            Stage::Vblank(241, 1) => {
                state.ppu.vblank = true;
                if state.ppu.is_nmi_enabled() {
                    state.cpu.nmi_req();
                }
            },
            _ => {}
        }

        match state.ppu.stage {
            Stage::Dot(s, c) => {
                self.render(system, state, c, s);
            },
            _ => {}
        }

        state.ppu.stage = state.ppu.stage.increment();
    }

    fn render(&self, system: &System, state: &mut SystemState, dot: u32, scanline: u32) {
        let color = (((state.ppu.low_bg_shift >> 15) & 0x1) |
            ((state.ppu.high_bg_shift >> 14) & 0x2)) as u16;
        let attr = ((((state.ppu.low_attr_shift >> 7) & 0x1) |
            ((state.ppu.high_attr_shift >> 6) & 0x2)) << 2) as u16;

        let palette = color | attr | 0x3f00;

        let bg_result = state.ppu.palette_data[(palette & 0x1f) as usize];
       

        state.ppu.screen[((scanline * 256) + dot) as usize] = bg_result;

        state.ppu.low_attr_shift <<= 1;
        state.ppu.high_attr_shift <<= 1;
        state.ppu.low_bg_shift <<= 1;
        state.ppu.high_bg_shift <<= 1;
    }

    fn horz_increment(&self, state: &mut SystemState) {
        let mut addr = state.ppu.vram_addr;
        if addr & 0x001f == 31 {
            addr &= !0x001f;
            addr ^= 0x0400;
        } else {
            addr += 1;
        }
        state.ppu.vram_addr = addr;
    }

    fn vert_increment(&self, state: &mut SystemState) {
        let mut addr = state.ppu.vram_addr;
        if (addr & 0x7000) != 0x7000 {
            addr += 0x1000;
        } else {
            addr &= !0x7000;
            let mut y = (addr & 0x03e0) >> 5;
            if y == 29 {
                y = 0;
                addr ^= 0x0800;
            } else if y == 31{
                y = 0;
            } else {
                y += 1;
            }

            addr = (addr & !0x03e0) | (y << 5);
        }
        state.ppu.vram_addr = addr;
    }

    fn horz_reset(&self, state: &mut SystemState) {
        let mut addr = state.ppu.vram_addr;
        let addr_t = state.ppu.vram_addr_temp;

        addr &= 0xfbe0;
        addr |= addr_t & 0x041f;
        state.ppu.vram_addr = addr;
    }

    fn vert_reset(&self, state: &mut SystemState) {
        let mut addr = state.ppu.vram_addr;
        let addr_t = state.ppu.vram_addr_temp;

        addr &= 0x841f;
        addr |= addr_t & 0x7be0;
        state.ppu.vram_addr = addr;
    }

    fn load_bg_shifters(&self, state: &mut SystemState) {
        state.ppu.low_bg_shift &= 0xff00;
        state.ppu.low_bg_shift |= state.ppu.pattern_low as u16;
        state.ppu.high_bg_shift &= 0xff00;
        state.ppu.high_bg_shift |= state.ppu.pattern_high as u16;

        state.ppu.low_attr_shift = (state.ppu.attribute_low & 1) * 0xff;
        state.ppu.high_attr_shift = (state.ppu.attribute_high & 1) * 0xff;

    }

    fn fetch_nametable(&self, system: &System, state: &mut SystemState) {
        let nt_addr = 0x2000 | (state.ppu.vram_addr & 0xfff);
        state.ppu.nametable_tile = self.bus.read(system, state, nt_addr);
    }

    fn fetch_attribute(&self, system: &System, state: &mut SystemState) {
        let v = state.ppu.vram_addr;
        let at_addr = 0x23c0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attr = self.bus.read(system, state, at_addr);

        let tile_num = state.ppu.vram_addr & 0xfff;
        let tile_x = tile_num % 16;
        let tile_y = tile_num / 16;

        let attr_quad = (tile_y % 2, tile_x % 2);
        match attr_quad {
            (0,0) => {
                state.ppu.attribute_low = (attr >> 0) & 1;
                state.ppu.attribute_high = (attr >> 1) & 1;
            },
            (0,1) => {
                state.ppu.attribute_low = (attr >> 2) & 1;
                state.ppu.attribute_high = (attr >> 3) & 1;
            },
            (1,0) => {
                state.ppu.attribute_low = (attr >> 4) & 1;
                state.ppu.attribute_high = (attr >> 5) & 1;
            },
            (1,1) => { 
                state.ppu.attribute_low = (attr >> 6) & 1;
                state.ppu.attribute_high = (attr >> 7) & 1;
            },
            _ => unreachable!()
        }
    }

    fn fetch_low_bg_pattern(&self, system: &System, state: &mut SystemState) {
        let v = state.ppu.vram_addr;
        let tile_addr = ((v >> 12) & 0x07) | ((state.ppu.nametable_tile as u16) << 4) |
            state.ppu.background_pattern_table();
        state.ppu.pattern_low = self.bus.read(system, state, tile_addr);
    }
    
    fn fetch_high_bg_pattern(&self, system: &System, state: &mut SystemState) {
        let v = state.ppu.vram_addr;
        let tile_addr = ((v >> 12) & 0x07) | ((state.ppu.nametable_tile as u16) << 4) |
            state.ppu.background_pattern_table() | 0x08;
        state.ppu.pattern_high = self.bus.read(system, state, tile_addr);
    }
}
