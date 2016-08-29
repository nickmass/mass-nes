use std::rc::Rc;
use nes::bus::{AddressValidator, AddressBus, BusKind, DeviceKind, Address};
use nes::system::{Region, SystemState, System};
use nes::memory::MemoryBlock;
use nes::nametables::{NametablesState, Nametables};

pub struct PpuState {
    current_tick: u64,
    last_status_read: u64,
    last_nmi_set: u64,
    pub frame: u32,
    regs: [u8;8],
    vblank: bool,
    pub in_vblank: bool,
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
    line_oam_data: [u8; 32],

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

    low_attr_shift: u16,
    high_attr_shift: u16,

    pub screen: [u8;256*240],

    in_sprite_render: bool,
    next_sprite_byte: u8,
    sprite_n: u32,
    sprite_m: u32,
    sprite_read_loop: bool,
    block_oam_writes: bool,
    found_sprites: u32,
    sprite_reads: u32,
    line_oam_index: usize,
    sprite_zero_on_line: bool,
    sprite_zero_on_next_line: bool,
    
    sprite_active: [u8; 8],
    sprite_x: [u8; 8],
    sprite_attr: [u8; 8],
    sprite_pattern_high: [u8; 8],
    sprite_pattern_low: [u8; 8],
    sprite_render_index: usize,

    pub nametables: NametablesState,
}

impl Default for PpuState {
    fn default() -> PpuState {
        PpuState {
            current_tick: 0,
            last_status_read: 0,
            last_nmi_set: 0,
            frame: 0,
            regs: [0;8],
            vblank: false,
            in_vblank: false,
            sprite_zero_hit: false,
            sprite_overflow: false,
            last_write: 0,

            write_latch: false,

            data_read_buffer: 0,

            vram_addr: 0,
            vram_addr_temp: 0,
            vram_fine_x: 0,

            oam_addr: 0,
            oam_data: [0; 256],
            line_oam_data: [0; 32],

            current_frame: 0,
            current_line: 0,
            stage: Stage::Dot(0,0),

            palette_data: [0; 32],

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

            in_sprite_render: false,
            next_sprite_byte: 0,
            sprite_n: 0,
            sprite_m: 0,
            sprite_read_loop: false,
            block_oam_writes: false,
            found_sprites: 0,
            sprite_reads: 0,
            line_oam_index: 0,
            sprite_zero_on_line: false,
            sprite_zero_on_next_line: false,

            sprite_active: [0; 8],
            sprite_x: [0; 8],
            sprite_attr: [0; 8],
            sprite_pattern_high: [0; 8],
            sprite_pattern_low: [0; 8],
            sprite_render_index: 0,

            nametables: Default::default(),
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
        if self.regs[0] & 0x04 != 0 { 0x20 } else { 0x01 }
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
    fn is_left_background(&self) -> bool { self.regs[1] & 0x02 != 0 }
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
                if d == 340 {
                    Stage::Dot(0, 0)
                } else {
                    Stage::Prerender(s, d + 1)
                }
            },
            Stage::Vblank(s, d) => {
                if d == 340 {
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
                if d == 340 {
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
    bus: AddressBus,
    pub nametables: Nametables,
}

impl Ppu {
    pub fn new(region: Region, state: &mut SystemState) -> Ppu {
        let ppu = Ppu {
            region: region,
            bus: AddressBus::new(BusKind::Ppu, state, 0),
            nametables: Nametables::new(state),
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
            0x2000 => state.ppu.last_write,
            0x2001 => state.ppu.last_write,
            0x2002 => state.ppu.ppu_status(),
            0x2003 => state.ppu.oam_addr, //OAMADDR
            0x2004 => state.ppu.oam_data[state.ppu.oam_addr as usize], //OANDATA
            0x2005 => state.ppu.last_write,
            0x2006 => state.ppu.last_write,
            0x2007 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    let addr  = if addr & 0x03 != 0 { addr & 0x1f } else { addr & 0x0f };
                    if state.ppu.is_grayscale() {
                        state.ppu.palette_data[addr as usize] & 0x30
                    } else {
                        state.ppu.palette_data[addr as usize]
                    }
                } else {
                    state.ppu.data_read_buffer
                };
                result
            }
            0x4014 => 0,
            _ => unreachable!(),
        }
    }
    pub fn read(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16) -> u8 {
        match address {
            0x2000 => state.ppu.last_write,
            0x2001 => state.ppu.last_write,
            0x2002 => {  
                let status = state.ppu.ppu_status();
                state.ppu.write_latch = false;
                state.ppu.vblank = false;
                state.ppu.last_status_read = state.ppu.current_tick;
                if state.ppu.last_nmi_set == state.ppu.current_tick ||
                   state.ppu.last_nmi_set == state.ppu.current_tick - 1{
                    state.cpu.nmi_cancel();
                }
                status
            }, //PPUSTATUS
            0x2003 => state.ppu.oam_addr, //OAMADDR
            0x2004 => {
                if state.ppu.in_sprite_render {
                    0xff
                } else {
                    state.ppu.oam_data[state.ppu.oam_addr as usize]
                }
            }, //OANDATA
            0x2005 => state.ppu.last_write,
            0x2006 => state.ppu.last_write,
            0x2007 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                let result = if addr & 0x3f00 == 0x3f00 {
                    let addr  = if addr & 0x03 != 0 { addr & 0x1f } else { addr & 0x0f };
                    if state.ppu.is_grayscale() {
                        state.ppu.palette_data[addr as usize] & 0x30
                    } else {
                        state.ppu.palette_data[addr as usize]
                    }
                } else {
                    state.ppu.data_read_buffer
                };
                state.ppu.data_read_buffer = self.bus.read(system, state, addr);
                state.ppu.vram_addr = state.ppu.vram_addr
                            .wrapping_add(state.ppu.vram_inc()) & 0x7fff;
                result
            }
            0x4014 => 0,
            _ => unreachable!(),
        }
    }

    pub fn write(&self, bus: BusKind, system: &System, state: &mut SystemState, address: u16, value: u8) {
        match address {
            0x2000 => {
                let was_nmi_enabled = state.ppu.is_nmi_enabled();
                state.ppu.regs[0] = value;
                state.ppu.vram_addr_temp &= 0xf3ff;
                state.ppu.vram_addr_temp |= state.ppu.base_nametable();
                match state.ppu.stage {
                    Stage::Vblank(_, _) => {
                        if !was_nmi_enabled && state.ppu.is_nmi_enabled() && 
                            state.ppu.vblank {
                            state.ppu.last_nmi_set = state.ppu.current_tick;
                            state.cpu.nmi_req(1);
                        }
                    },
                    _ => {}
                }
            }, //PPUCTRL
            0x2001 => state.ppu.regs[1] = value, //PPUMASK
            0x2002 => state.ppu.regs[2] = value,
            0x2003 => state.ppu.oam_addr = value, //OAMADDR
            0x2004 => {
                state.ppu.oam_data[state.ppu.oam_addr as usize] = value;
                state.ppu.oam_addr = state.ppu.oam_addr.wrapping_add(1);
            }, //OAMDATA
            0x2005 => { //PPUSCROLL
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
            0x2006 => { //PPUADDR
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
            0x2007 => { //PPUDATA
                let addr = state.ppu.vram_addr;
                if addr & 0x3f00 == 0x3f00 {
                    let addr  = if addr & 0x03 != 0 { addr & 0x1f } else { addr & 0x0f };
                    state.ppu.palette_data[addr as usize] = value;
                } else {
                    self.bus.write(system, state, addr & 0x3fff, value);
                }
                state.ppu.vram_addr = state.ppu.vram_addr
                    .wrapping_add(state.ppu.vram_inc()) & 0x7fff;
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
            Stage::Prerender(261, 1) => {
                state.ppu.in_vblank = false;
                state.ppu.vblank = false;
                state.ppu.sprite_zero_hit = false;
                state.ppu.sprite_overflow = false;
                state.ppu.frame += 1;
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
                self.load_bg_shifters(state);
                self.horz_increment(state);
            },
            Stage::Prerender(s, 256) => {
                self.horz_increment(state);
                self.vert_increment(state);
            },
            Stage::Prerender(s, 257) => {
                self.horz_reset(state);
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
                self.load_bg_shifters(state);
                self.horz_increment(state);
            },
            Stage::Prerender(s, 340) => {
                //Skip tick on odd frames
                if state.ppu.frame % 2 == 1 && state.ppu.is_background_enabled() {
                    state.ppu.stage = state.ppu.stage.increment();
                }
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
                self.load_bg_shifters(state);
                self.horz_increment(state);
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
                self.load_bg_shifters(state);
                self.horz_increment(state);
            },
            Stage::Hblank(s, 256) => {
                self.horz_increment(state);
                self.vert_increment(state);
            },
            Stage::Hblank(s, 257) => {
                self.horz_reset(state);
            },
            Stage::Vblank(241, 1) => {
                state.ppu.in_vblank = true;
                if state.ppu.current_tick != state.ppu.last_status_read + 1 {
                    state.ppu.vblank = true;
                    if state.ppu.is_nmi_enabled() {
                        state.cpu.nmi_req(1);
                        state.ppu.last_nmi_set = state.ppu.current_tick;
                    }
                }
            },
            _ => {}
        }

        match state.ppu.stage {
            Stage::Dot(s, 1) => {
                state.ppu.sprite_render_index = 0;
                state.ppu.sprite_n = 0;
                state.ppu.sprite_m = 0;
                state.ppu.found_sprites = 0;
                state.ppu.sprite_reads = 0;
                state.ppu.line_oam_index = 0;
                state.ppu.in_sprite_render = false;
                state.ppu.sprite_read_loop = false;
                state.ppu.block_oam_writes = false;
                state.ppu.sprite_zero_on_line = state.ppu.sprite_zero_on_next_line;
                state.ppu.sprite_zero_on_next_line = false;
                self.init_line_oam(system, state, 0);
            },
            Stage::Dot(s, d) if d >= 1 && d < 65 && d % 2 == 1 => {
                state.ppu.in_sprite_render = false;
                self.init_line_oam(system, state, d / 2);
            },
            Stage::Hblank(s, 256) => {
                state.ppu.sprite_n = 0;
                self.sprite_eval(system, state, s);
            },
            Stage::Dot(s, d) if d >= 65 && d % 2 == 0 => {
                self.sprite_eval(system, state, s);
            },
            Stage::Dot(s, d) if d >= 65 && d % 2 == 1 => {
                state.ppu.in_sprite_render = false;
                self.sprite_read(system, state);
            },
            Stage::Hblank(s, d) if d >=257 && d < 320 && d % 8 == 1 => {
                //Garbage Nametable
                self.fetch_nametable(system, state);
            },
            Stage::Hblank(s, d) if d >=257 && d < 320 && d % 8 == 3 => {
               //Garbage Nametable 
                self.fetch_attribute(system, state);
            },
            Stage::Hblank(s, d) if d >=257 && d < 320 && d % 8 == 5 => {
                self.sprite_fetch(system, state, s, false);
            },
            Stage::Hblank(s, d) if d >=257 && d < 320 && d % 8 == 7 => {
                self.sprite_fetch(system, state, s, true);
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
        let fine_x = state.ppu.vram_fine_x;
        let color = (((state.ppu.low_bg_shift >> (15 - fine_x)) & 0x1) |
            ((state.ppu.high_bg_shift >> (14 - fine_x)) & 0x2)) as u16;
        
        let high_attr = state.ppu.high_attr_shift as u32;
        let low_attr = state.ppu.low_attr_shift as u32;

        let high_attr = high_attr << fine_x;
        let low_attr = low_attr << fine_x;

        let high_attr = (high_attr >> 14) & 0x02;
        let low_attr = (low_attr >> 15) & 0x01;

        let attr = ((high_attr | low_attr) << 2) as u16;

        let attr = if color == 0 { 0 } else { attr };

        let palette = color | attr;

        let mut sprite_zero = false;
        let mut sprite_pixel = 0;
        let mut behind_bg = false;
        let mut x = 8;
        loop {
            x -= 1;
            if state.ppu.sprite_active[x] > 0 {
                if state.ppu.sprite_active[x] <= 8 {
                    let attr = state.ppu.sprite_attr[x];
                    let high = state.ppu.sprite_pattern_high[x];
                    let low = state.ppu.sprite_pattern_low[x];
                    let flip_horz  = attr & 0x40 != 0;
                    let pal = (attr & 0x3) << 2;

                    let pal_bit = if flip_horz { 0x1 } else {0x80};
                    let color = if high & pal_bit != 0 { 2 } else { 0 } |
                                if low & pal_bit != 0 { 1 } else { 0 };
                    
                    if color != 0 {
                        sprite_zero = x == 0 && state.ppu.sprite_zero_on_line;
                        sprite_pixel = color | pal;
                        behind_bg  = attr & 0x20 != 0;
                    }
                    
                    state.ppu.sprite_active[x] += 1;

                    if flip_horz {
                        state.ppu.sprite_pattern_high[x] >>= 1;
                        state.ppu.sprite_pattern_low[x] >>= 1;
                    } else {
                        state.ppu.sprite_pattern_high[x] <<= 1;
                        state.ppu.sprite_pattern_low[x] <<= 1;
                    }
                }
            } else {
                if state.ppu.sprite_x[x] != 0 && state.ppu.sprite_x[x] != 0xff {
                    state.ppu.sprite_x[x] -= 1;
                }
                if state.ppu.sprite_x[x] == 0 {
                    if dot != 254 {
                    }
                    state.ppu.sprite_active[x] = 1;
                }
            }
            if x == 0 { break; }
        }

        let bg_colored = color != 0 && (dot > 7 || state.ppu.is_left_background());
        let sprite_colored = sprite_pixel != 0 && 
            (dot > 7 || state.ppu.is_left_sprites());
        
        let pixel = match (bg_colored, sprite_colored, behind_bg) {
            (false, false, _) => 0x3f00,
            (false, true, _) => 0x3f10 | sprite_pixel as u16,
            (true, false, _) => 0x3f00 | palette as u16,
            (true, true, false) => {
                if sprite_zero { state.ppu.sprite_zero_hit = true; }
                0x3f10 | sprite_pixel as u16 },
            (true, true, true) => { 
                if sprite_zero { state.ppu.sprite_zero_hit = true; }
                0x3f00 | palette as u16 },
        };

        let addr  = if pixel & 0x03 != 0 { pixel & 0x1f } else { pixel & 0x0f };
        let mut pixel_result = state.ppu.palette_data[addr as usize];
        
        if state.ppu.is_grayscale() {
            pixel_result &= 0x30;
        }
       
        if system.debug.color(state) {
            pixel_result = 0x14
        }

        //TODO - Do emphasis bits
        state.ppu.screen[((scanline * 256) + dot) as usize] = pixel_result;

        state.ppu.low_attr_shift <<= 1;
        state.ppu.high_attr_shift <<= 1;
        state.ppu.low_bg_shift <<= 1;
        state.ppu.high_bg_shift <<= 1;
    }
    fn sprite_fetch(&self, system: &System, state: &mut SystemState, scanline: u32, high: bool) {
        if !state.ppu.is_sprites_enabled() { return; }
        let is_tall = state.ppu.is_tall_sprites();
        let is_on_line = |sprite_y, scanline| {
            if is_tall {
                (sprite_y as u32)+ 16 > scanline && (sprite_y as u32) <= scanline
            } else {
                (sprite_y as u32)+ 8 > scanline && (sprite_y as u32) <= scanline
            }
        };
        let index = state.ppu.sprite_render_index;
        let sprite_y = state.ppu.line_oam_data[(index * 4)];
        let sprite_tile = state.ppu.line_oam_data[(index * 4) + 1] as u16;
        let sprite_attr = state.ppu.line_oam_data[(index * 4) + 2];
        let sprite_x = state.ppu.line_oam_data[(index * 4) + 3];
        
        let flip_vert  = sprite_attr & 0x80 != 0;
        let line = if scanline >= sprite_y as u32 {
            (scanline - sprite_y as u32) as u16
        } else {
            0
        };
        let line = if line > 8 { 0 } else { line };
        let tile_addr = if state.ppu.is_tall_sprites() {
            let bottom_half = line >= 8;
            let line = if bottom_half { line - 8 } else { line };
            let line = if flip_vert { 7 - line } else { line };
            let pattern_table = (sprite_tile as u16 & 1) << 12;
            let sprite_tile = sprite_tile & 0xfe;

            match (flip_vert, bottom_half) {
                (true , true ) | (false, false) => 
                    ((sprite_tile << 4) | pattern_table) + line,
                (false, true) | (true, false) => 
                    (((sprite_tile + 1) << 4) | pattern_table) + line,
            }
        } else {
            let line = if flip_vert { 7 - line } else { line };
            ((sprite_tile << 4) | state.ppu.sprite_pattern_table()) + line
        };

        state.ppu.sprite_x[index] = sprite_x;
        state.ppu.sprite_attr[index] = sprite_attr;
        state.ppu.sprite_active[index] = 0;
        if high {
            state.ppu.sprite_pattern_high[index]
                = self.bus.read(system, state, tile_addr | 0x08);
            if !is_on_line(sprite_y, scanline){state.ppu.sprite_pattern_high[index] = 0};
            state.ppu.sprite_render_index += 1;
        } else {
            state.ppu.sprite_pattern_low[index]
                = self.bus.read(system, state, tile_addr);
            if !is_on_line(sprite_y, scanline){state.ppu.sprite_pattern_low[index] = 0};
        }
    }

    fn sprite_read(&self, system: &System, state: &mut SystemState) {
        if !state.ppu.is_sprites_enabled() { return; }
        state.ppu.next_sprite_byte = state.ppu.oam_data[((state.ppu.sprite_n * 4) + state.ppu.sprite_m) as usize];
    }

    fn sprite_eval(&self, system: &System, state: &mut SystemState, scanline: u32) {
        if !state.ppu.is_sprites_enabled() { return; }
        if state.ppu.sprite_read_loop { return; }
        let is_tall = state.ppu.is_tall_sprites();
        let is_on_line = |sprite_y, scanline| {
            if is_tall {
                (sprite_y as u32)+ 16 > scanline && (sprite_y as u32) <= scanline
            } else {
                (sprite_y as u32)+ 8 > scanline && (sprite_y as u32) <= scanline
            }
        };

        if !state.ppu.block_oam_writes {
            state.ppu.line_oam_data[state.ppu.line_oam_index] =
                state.ppu.next_sprite_byte;
        }
        if state.ppu.found_sprites == 8 {
            if state.ppu.sprite_reads != 0 {
                state.ppu.sprite_m += 1;
                state.ppu.sprite_m &= 3;
                if state.ppu.sprite_m == 0 {
                    state.ppu.sprite_n += 1;
                    if state.ppu.sprite_n == 64 {
                        state.ppu.sprite_read_loop = true;
                        state.ppu.sprite_n = 0;
                        state.ppu.sprite_m = 0;
                    }
                }
                state.ppu.sprite_reads -= 1;
            } else {
                if is_on_line(state.ppu.next_sprite_byte, scanline) {
                        state.ppu.sprite_overflow = true;
                        state.ppu.sprite_m += 1;
                        state.ppu.sprite_m &= 3;
                        state.ppu.sprite_reads = 3;
                    } else {
                        state.ppu.sprite_n += 1;
                        state.ppu.sprite_m += 1;
                        state.ppu.sprite_m &= 3;
                        if state.ppu.sprite_n == 64 {
                            state.ppu.sprite_read_loop = true;
                            state.ppu.sprite_n = 0;
                        }
                    }
            }
        } else { //Less then 8 sprites found
            if state.ppu.sprite_reads != 0 {
                state.ppu.sprite_m += 1;
                state.ppu.sprite_m &= 3;
                state.ppu.line_oam_index += 1;
                state.ppu.sprite_reads -= 1;    
            } else if is_on_line(state.ppu.next_sprite_byte, scanline) {
                if state.ppu.sprite_n == 0 { state.ppu.sprite_zero_on_next_line = true; }
                state.ppu.sprite_m += 1;
                state.ppu.sprite_reads = 3;
                state.ppu.line_oam_index += 1;
                state.ppu.found_sprites += 1;
            }
            if state.ppu.sprite_reads == 0 {
                state.ppu.sprite_n += 1;
                state.ppu.sprite_m = 0;
                if state.ppu.sprite_n == 64 {
                    state.ppu.sprite_read_loop = true;
                    state.ppu.sprite_n = 0;
                } else if state.ppu.found_sprites == 8 {
                    state.ppu.block_oam_writes = true;
                }
            }
        }
    }

    fn init_line_oam(&self, system: &System, state: &mut SystemState, addr: u32) {
        if !state.ppu.is_sprites_enabled() { return; }
        state.ppu.in_sprite_render = true;
        state.ppu.line_oam_data[addr as usize] = 0xff;
    }

    fn horz_increment(&self, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
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
        if !state.ppu.is_background_enabled() { return; }
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
        if !state.ppu.is_background_enabled() { return; }
        let mut addr = state.ppu.vram_addr;
        let addr_t = state.ppu.vram_addr_temp;

        addr &= 0xfbe0;
        addr |= addr_t & 0x041f;
        state.ppu.vram_addr = addr;
    }

    fn vert_reset(&self, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
        let mut addr = state.ppu.vram_addr;
        let addr_t = state.ppu.vram_addr_temp;

        addr &= 0x841f;
        addr |= addr_t & 0x7be0;
        state.ppu.vram_addr = addr;
    }

    fn load_bg_shifters(&self, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
        state.ppu.low_bg_shift &= 0xff00;
        state.ppu.low_bg_shift |= state.ppu.pattern_low as u16;
        state.ppu.high_bg_shift &= 0xff00;
        state.ppu.high_bg_shift |= state.ppu.pattern_high as u16;

        state.ppu.low_attr_shift &= 0xff00;
        state.ppu.low_attr_shift |= ((state.ppu.attribute_low & 1) * 0xff) as u16;
        state.ppu.high_attr_shift &=0xff00;
        state.ppu.high_attr_shift |= ((state.ppu.attribute_high & 1) * 0xff) as u16;

    }

    fn fetch_nametable(&self, system: &System, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
        let nt_addr = 0x2000 | (state.ppu.vram_addr & 0xfff);
        state.ppu.nametable_tile = self.bus.read(system, state, nt_addr);
    }

    fn fetch_attribute(&self, system: &System, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
        let v = state.ppu.vram_addr;
        let at_addr = 0x23c0 | (v & 0x0c00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attr = self.bus.read(system, state, at_addr);

        let tile_num = state.ppu.vram_addr & 0x3ff;
        let tile_x = tile_num % 32;
        let tile_y = tile_num / 32;

        let attr_quad = ((tile_y >> 1) & 1, (tile_x >> 1) & 1);
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
        if !state.ppu.is_background_enabled() { return; }
        let v = state.ppu.vram_addr;
        let tile_addr = ((v >> 12) & 0x07) | ((state.ppu.nametable_tile as u16) << 4) |
            state.ppu.background_pattern_table();
        state.ppu.pattern_low = self.bus.read(system, state, tile_addr);
    }
    
    fn fetch_high_bg_pattern(&self, system: &System, state: &mut SystemState) {
        if !state.ppu.is_background_enabled() { return; }
        let v = state.ppu.vram_addr;
        let tile_addr = ((v >> 12) & 0x07) | ((state.ppu.nametable_tile as u16) << 4) |
            state.ppu.background_pattern_table() | 0x08;
        state.ppu.pattern_high = self.bus.read(system, state, tile_addr);
    }
}
