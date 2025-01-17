use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use super::{samples_channel, Audio, SamplesSender};

pub struct Null {
    sample_rate: u32,
    pause: Pause,
}

impl Null {
    pub fn new() -> (Self, SamplesSender) {
        let pause = Pause::new();
        let sample_rate = 48000;

        let (tx, mut rx) = samples_channel(sample_rate as usize, 1024);

        let inner_pause = pause.clone();

        std::thread::spawn(move || {
            let pause = inner_pause;
            let samples_per_ms = sample_rate as u64 / 1000;
            let mut now = Instant::now();
            let mut dur = Duration::ZERO;
            loop {
                if !pause.is_paused() {
                    dur += now.elapsed();
                    now = Instant::now();
                    let millis = dur.as_millis() as u64;
                    dur -= Duration::from_millis(millis);

                    let target_samples = samples_per_ms * millis;

                    for _ in (0..target_samples).zip(&mut rx) {}
                } else {
                    now = Instant::now();
                }

                std::thread::sleep(Duration::from_millis(1));
            }
        });

        (Self { pause, sample_rate }, tx)
    }
}

impl Audio for Null {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn play(&mut self) {
        self.pause.set(false);
    }
    fn pause(&mut self) {
        self.pause.set(true);
    }
    fn volume(&mut self, _volume: f32) {}
}

#[derive(Clone)]
struct Pause {
    pause: Arc<AtomicBool>,
}

impl Pause {
    fn new() -> Self {
        Self {
            pause: Arc::new(AtomicBool::new(true)),
        }
    }
    fn set(&self, paused: bool) {
        self.pause.store(paused, Ordering::Relaxed);
    }

    fn is_paused(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }
}
