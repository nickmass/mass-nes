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
    pub const fn frame_ticks(&self) -> f64 {
        match self {
            Region::Ntsc => 29780.5,
            Region::Pal => 33247.5,
        }
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
        match self {
            Region::Ntsc => 60.0988,
            Region::Pal => 50.007,
        }
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
