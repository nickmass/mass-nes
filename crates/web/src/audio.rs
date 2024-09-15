use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    StreamExt,
};

pub struct CpalSync {
    tx: Sender<()>,
    rx: Receiver<()>,
}

impl CpalSync {
    pub fn new() -> Self {
        let (tx, rx) = channel(1);
        Self { tx, rx }
    }

    async fn sync(&mut self) {
        let _ = self.rx.next().await;
    }
}

impl ui::audio::Parker for CpalSync {
    type Unparker = CpalUnparker;

    fn unparker(&self) -> Self::Unparker {
        CpalUnparker {
            tx: self.tx.clone(),
        }
    }
}

pub struct CpalUnparker {
    tx: Sender<()>,
}

impl ui::audio::Unparker for CpalUnparker {
    fn unpark(&mut self) {
        let _ = self.tx.try_send(());
    }
}

impl super::sync::FrameSync for CpalSync {
    fn sync_frame(&mut self) -> impl std::future::Future<Output = ()> {
        self.sync()
    }
}
