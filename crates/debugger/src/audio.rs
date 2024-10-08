use crossbeam::sync::{Parker, Unparker};

pub struct CpalSync {
    parker: Parker,
}

impl CpalSync {
    pub fn new() -> Self {
        Self {
            parker: Parker::new(),
        }
    }
}

impl ui::audio::Parker for CpalSync {
    type Unparker = CpalUnparker;

    fn unparker(&self) -> Self::Unparker {
        CpalUnparker(self.parker.unparker().clone())
    }
}

pub struct CpalUnparker(Unparker);

impl ui::audio::Unparker for CpalUnparker {
    fn unpark(&mut self) {
        self.0.unpark()
    }
}

impl FrameSync for CpalSync {
    fn sync_frame(&mut self) {
        self.parker.park()
    }
}

pub trait FrameSync: Send + 'static {
    fn sync_frame(&mut self);
}
