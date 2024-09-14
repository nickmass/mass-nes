use futures::{Future, StreamExt};
use gloo::timers::future::IntervalStream;

pub struct NaiveSync {
    timer: IntervalStream,
}

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

pub enum SyncDevices {
    //Cpal(super::audio::CpalSync),
    Naive(NaiveSync),
}

impl FrameSync for SyncDevices {
    fn sync_frame(&mut self) -> impl Future<Output = ()> {
        match self {
            //SyncDevices::Cpal(s) => s.sync_frame(),
            SyncDevices::Naive(s) => s.sync_frame(),
        }
    }
}

/*
impl From<super::audio::CpalSync> for SyncDevices {
    fn from(value: super::audio::CpalSync) -> Self {
        SyncDevices::Cpal(value)
    }
}
*/

impl From<NaiveSync> for SyncDevices {
    fn from(value: NaiveSync) -> Self {
        SyncDevices::Naive(value)
    }
}
