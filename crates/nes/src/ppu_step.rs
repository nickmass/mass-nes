#[cfg(feature = "save-states")]
use serde::{Deserialize, Serialize};

use crate::region::Region;

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum BackgroundStep {
    VertReset,
    HorzReset,
    VertIncrement,
    HorzIncrement,
    ShiftedHorzIncrement,
    Nametable,
    Attribute,
    LowPattern,
    HighPattern,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum SpriteStep {
    Clear,
    Eval,
    Read,
    Reset,
    Hblank,
    Fetch(u8),
    BackgroundWait,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum StateChange {
    SkippedTick,
    SetVblank,
    ClearVblank,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone)]
pub struct PpuStep {
    pub background: Option<BackgroundStep>,
    pub sprite: Option<SpriteStep>,
    pub state: Option<StateChange>,
    pub scanline: u32,
    pub dot: u32,
}

#[cfg_attr(feature = "save-states", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct PpuSteps {
    index: usize,
    steps: Vec<PpuStep>,
}

impl PpuSteps {
    fn new(steps: Vec<PpuStep>) -> PpuSteps {
        PpuSteps { index: 0, steps }
    }

    pub fn step(&mut self) -> PpuStep {
        if self.index >= self.steps.len() {
            self.index = 0;
        }
        let next = self.steps[self.index];
        self.index += 1;

        next
    }
}

pub fn generate_steps(region: Region) -> PpuSteps {
    let prerender = region.prerender_line();
    let vblank = region.vblank_line();
    let skip = region.uneven_frames();
    let mut dot = 0;
    let mut scanline = 0;

    let mut steps = Vec::new();

    loop {
        let state = if scanline == prerender && dot == 1 {
            Some(StateChange::ClearVblank)
        } else if scanline == prerender && dot == 340 && skip {
            Some(StateChange::SkippedTick)
        } else if dot == 1 && scanline == vblank + 1 {
            Some(StateChange::SetVblank)
        } else {
            None
        };

        let background = if scanline == prerender && dot >= 280 && dot < 304 {
            Some(BackgroundStep::VertReset)
        } else if scanline >= vblank && scanline < prerender {
            None
        } else {
            match dot {
                c if c % 8 == 1 && c < 256 => Some(BackgroundStep::Nametable),
                c if c % 8 == 3 && c < 256 => Some(BackgroundStep::Attribute),
                c if c % 8 == 5 && c < 256 => Some(BackgroundStep::LowPattern),
                c if c % 8 == 7 && c < 256 => Some(BackgroundStep::HighPattern),
                c if c % 8 == 0 && c != 0 && c < 256 => Some(BackgroundStep::HorzIncrement),
                256 => Some(BackgroundStep::VertIncrement),
                257 => Some(BackgroundStep::HorzReset),
                c if c == 321 || c == 329 || c == 337 || c == 339 => {
                    Some(BackgroundStep::Nametable)
                }
                c if c == 323 || c == 331 => Some(BackgroundStep::Attribute),
                c if c == 325 || c == 333 => Some(BackgroundStep::LowPattern),
                c if c == 327 || c == 335 => Some(BackgroundStep::HighPattern),
                c if c == 328 => Some(BackgroundStep::HorzIncrement),
                c if c == 336 => Some(BackgroundStep::ShiftedHorzIncrement),
                _ => None,
            }
        };

        let sprite = if scanline == prerender || scanline < vblank {
            match dot {
                0 => Some(SpriteStep::Reset),
                d if d >= 1 && d < 65 && d & 1 == 1 => Some(SpriteStep::Clear),
                d if d >= 65 && d < 256 => {
                    if d & 1 == 0 {
                        Some(SpriteStep::Eval)
                    } else {
                        Some(SpriteStep::Read)
                    }
                }
                256 => Some(SpriteStep::Hblank),
                d if d >= 257 && d < 320 => Some(SpriteStep::Fetch((d % 8) as u8)),
                d if d >= 321 && d < 340 => Some(SpriteStep::BackgroundWait),
                _ => None,
            }
        } else {
            None
        };

        steps.push(PpuStep {
            background,
            sprite,
            state,
            scanline,
            dot,
        });

        dot += 1;
        if dot == 341 {
            dot = 0;
            if scanline == prerender {
                break;
            } else {
                scanline += 1;
            }
        }
    }

    PpuSteps::new(steps)
}
