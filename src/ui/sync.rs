use std::time::Duration;
use std::thread::{spawn, sleep, JoinHandle};
pub struct FrameSync {
    refresh_rate: f64,
    frame: Option<Frame>,
}

impl FrameSync {
    pub fn new(refresh_rate: f64) -> FrameSync {
        FrameSync {
            refresh_rate: refresh_rate,
            frame: None,
        }
    }

    pub fn begin_frame(&mut self) {
        let sleep_time = Duration::new(0, ((1.0 / self.refresh_rate) * 1000000000.0) as u32);
        let t = spawn(move || {
            sleep(sleep_time);
        });

        let frame = Frame {
            handle: t
        };

        self.frame = Some(frame);
    }

    pub fn end_frame(&mut self) {
        let frame = ::std::mem::replace(&mut self.frame, None);
        frame.unwrap().end_frame();
    }
}


pub struct Frame {
    handle: JoinHandle<()>,
}

impl Frame {
    fn end_frame(self) {
        let _ = self.handle.join();
    }
}
