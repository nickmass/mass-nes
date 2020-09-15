use std::thread;
use std::time::{Duration, Instant};

pub struct FrameSync {
    frame_ns: u32,
    offset_ns: i64,
    compute_start: Instant,
}

impl FrameSync {
    pub fn new(refresh_rate: f64) -> FrameSync {
        let frame_ns = ((1.0 / refresh_rate) * 1000000000.0) as u32;
        FrameSync {
            frame_ns,
            offset_ns: 0,
            compute_start: Instant::now(),
        }
    }

    pub fn sync_frame(&mut self) {
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
            thread::sleep(Duration::new(0, delay - dur.subsec_nanos()));
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
