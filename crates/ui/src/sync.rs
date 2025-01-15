use crate::audio::SamplesSync;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub trait FrameSync: Send + 'static {
    fn sync_frame(&mut self);
}

#[cfg(not(target_arch = "wasm32"))]
impl FrameSync for SamplesSync {
    fn sync_frame(&mut self) {
        self.wait_for_need_samples();
    }
}

#[cfg(target_arch = "wasm32")]
impl FrameSync for SamplesSync {
    fn sync_frame(&mut self) {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1));
            if !self.need_samples() {
                continue;
            }
            break;
        }
    }
}

pub struct NaiveSync {
    frame_ns: u32,
    offset_ns: i64,
    compute_start: Instant,
}

impl NaiveSync {
    pub fn new(refresh_rate: f64) -> NaiveSync {
        let frame_ns = ((1.0 / refresh_rate) * 1000000000.0) as u32;
        NaiveSync {
            frame_ns,
            offset_ns: 0,
            compute_start: Instant::now(),
        }
    }
}

impl FrameSync for NaiveSync {
    fn sync_frame(&mut self) {
        let dur = self.compute_start.elapsed();
        let delay = self.frame_ns as i64 + self.offset_ns;
        let delay = if delay < 0 {
            0
        } else if delay > u32::max_value() as i64 {
            u32::max_value()
        } else {
            delay as u32
        };
        if dur.as_secs() == 0 && delay >= dur.subsec_nanos() {
            std::thread::sleep(Duration::new(0, delay - dur.subsec_nanos()));
            while delay > self.compute_start.elapsed().subsec_nanos() {}
        }
        let dur = self.compute_start.elapsed();
        if dur.as_secs() > 0 {
            self.offset_ns = -1000000000;
        } else {
            self.offset_ns = delay as i64 - dur.subsec_nanos() as i64;
        }
        self.compute_start = Instant::now();
    }
}

pub enum SyncDevices {
    Samples(SamplesSync),
    Naive(NaiveSync),
}

impl FrameSync for SyncDevices {
    fn sync_frame(&mut self) {
        match self {
            SyncDevices::Samples(s) => s.sync_frame(),
            SyncDevices::Naive(s) => s.sync_frame(),
        }
    }
}

impl From<SamplesSync> for SyncDevices {
    fn from(value: SamplesSync) -> Self {
        SyncDevices::Samples(value)
    }
}

impl From<NaiveSync> for SyncDevices {
    fn from(value: NaiveSync) -> Self {
        SyncDevices::Naive(value)
    }
}

#[derive(Debug, Clone)]
pub struct EmuSync {
    run: Arc<AtomicBool>,
}

impl EmuSync {
    pub fn new() -> Self {
        Self {
            run: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run(&self) -> bool {
        self.run.swap(false, Ordering::Relaxed)
    }

    pub fn request_run(&self) -> bool {
        self.run.swap(true, Ordering::Relaxed)
    }

    pub fn pending_run(&self) -> bool {
        self.run.load(Ordering::Relaxed)
    }
}
