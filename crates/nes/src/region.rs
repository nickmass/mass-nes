#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    pub const fn master_clock(&self) -> f64 {
        match self {
            Region::Ntsc => 236250000.0 / 11.0,
            Region::Pal => 26601712.5,
        }
    }

    pub const fn cpu_clock(&self) -> f64 {
        match self {
            Region::Ntsc => self.master_clock() / 12.0,
            Region::Pal => self.master_clock() / 16.0,
        }
    }

    pub const fn ppu_clock(&self) -> f64 {
        match self {
            Region::Ntsc => self.master_clock() / 4.0,
            Region::Pal => self.master_clock() / 5.0,
        }
    }

    pub const fn dots_per_tick(&self) -> f64 {
        self.ppu_clock() / self.cpu_clock()
    }

    pub const fn frame_dots(&self) -> f64 {
        let ticks = self.prerender_line() as f64 * 341.0;
        if self.uneven_frames() {
            ticks + 340.5
        } else {
            ticks + 341.0
        }
    }

    pub const fn frame_ticks(&self) -> f64 {
        self.frame_dots() / self.dots_per_tick()
    }

    pub const fn default_palette(&self) -> &'static [u8; 1536] {
        match self {
            Region::Ntsc => include_bytes!("default.pal"),
            Region::Pal => include_bytes!("default.pal"),
        }
    }

    pub const fn vblank_line(&self) -> u32 {
        match self {
            Region::Ntsc => 240,
            Region::Pal => 239,
        }
    }

    pub const fn prerender_line(&self) -> u32 {
        match self {
            Region::Ntsc => 261,
            Region::Pal => 311,
        }
    }

    pub const fn uneven_frames(&self) -> bool {
        match self {
            Region::Ntsc => true,
            Region::Pal => false,
        }
    }

    pub const fn emph_bits(&self) -> EmphMode {
        match self {
            Region::Ntsc => EmphMode::Bgr,
            Region::Pal => EmphMode::Brg,
        }
    }

    pub const fn extra_ppu_tick(&self) -> bool {
        match self {
            Region::Ntsc => false,
            Region::Pal => true,
        }
    }

    pub const fn refresh_rate(&self) -> f64 {
        self.cpu_clock() / self.frame_ticks()
    }

    pub const fn five_step_seq(&self) -> &'static [u32] {
        match self {
            Region::Ntsc => FIVE_STEP_SEQ_NTSC,
            Region::Pal => FIVE_STEP_SEQ_PAL,
        }
    }

    pub const fn four_step_seq(&self) -> &'static [u32] {
        match self {
            Region::Ntsc => FOUR_STEP_SEQ_NTSC,
            Region::Pal => FOUR_STEP_SEQ_PAL,
        }
    }

    pub const fn noise_rates(&self) -> &'static [u16] {
        match self {
            Region::Ntsc => NOISE_RATES_NTSC,
            Region::Pal => NOISE_RATES_PAL,
        }
    }

    pub const fn dmc_rates(&self) -> &'static [u16] {
        match self {
            Region::Ntsc => DMC_RATES_NTSC,
            Region::Pal => DMC_RATES_PAL,
        }
    }

    pub const fn dma_halt_on_read(&self) -> bool {
        match self {
            Region::Ntsc => true,
            Region::Pal => false,
        }
    }
}

const FIVE_STEP_SEQ_NTSC: &[u32] = &[7457, 14913, 22371, 37281, 37282];
const FIVE_STEP_SEQ_PAL: &[u32] = &[8313, 16627, 24939, 41565, 41566];

const FOUR_STEP_SEQ_NTSC: &[u32] = &[7457, 14913, 22371, 29829, 29830];
const FOUR_STEP_SEQ_PAL: &[u32] = &[8313, 16627, 24939, 33253, 33254];

const NOISE_RATES_NTSC: &[u16] = &[
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];
const NOISE_RATES_PAL: &[u16] = &[
    4, 8, 14, 30, 60, 88, 118, 148, 188, 236, 354, 472, 708, 944, 1890, 3778,
];

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
