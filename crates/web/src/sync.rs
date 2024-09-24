use futures::{Future, StreamExt};
use gloo::timers::future::IntervalStream;

pub struct NaiveSync {
    timer: IntervalStream,
}

#[allow(dead_code)]
impl NaiveSync {
    pub fn new(refresh_rate: f64) -> NaiveSync {
        NaiveSync {
            timer: IntervalStream::new(((1.0 / refresh_rate) * 1000.0) as u32),
        }
    }

    async fn wait(&mut self) {
        self.timer.next().await.unwrap();
    }
}

impl FrameSync for NaiveSync {
    fn sync_frame(&mut self) -> impl Future<Output = ()> {
        self.wait()
    }
}

pub trait FrameSync: 'static {
    fn sync_frame(&mut self) -> impl Future<Output = ()>;
}
