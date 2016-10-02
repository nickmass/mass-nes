extern crate cpal;
extern crate futures;

use self::futures::stream::Stream;
use self::futures::task;
use self::futures::task::{Run, Executor};

use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;

struct AudioExecutor;

impl Executor for AudioExecutor {
    fn execute(&self, r: Run) {
        r.run();
    }
}

pub struct Audio {
    endpoint: cpal::Endpoint,
    format: cpal::Format,
    voice: cpal::Voice,
    tx: Sender<Vec<i16>>,
}

impl Audio {
    pub fn new() -> Audio {
        let allowed_sample_rates = vec![cpal::SamplesRate(48000), cpal::SamplesRate(44100), cpal::SamplesRate(96000)];
        let endpoint = cpal::get_default_endpoint().unwrap();

        struct Match {
            sr: usize,
            chan: usize,
            data_type: cpal::SampleFormat,
            format: cpal::Format,
        }

        let mut best_match = None;
        let formats = endpoint.get_supported_formats_list().expect("No audio formats found");
        for f in formats {
            let s = allowed_sample_rates.iter().position(|x| *x == f.samples_rate); 
            let c = f.channels.len();
            let d = f.data_type;
            if s.is_none() { continue; }
            let s = s.unwrap();
            let new_m = Match {sr: s, chan: c, data_type: d, format: f.clone() };
            if best_match.is_none() {
                best_match = Some(new_m);
                continue;
            }
            if best_match.as_ref().unwrap().sr > s {
                best_match = Some(new_m);
                continue;
            }
            if best_match.as_ref().unwrap().sr < s {
                continue;
            }
            if best_match.as_ref().unwrap().chan > c {
                best_match = Some(new_m);
                continue;
            }
            if best_match.as_ref().unwrap().chan < c {
                continue;
            }
            if d == cpal::SampleFormat::I16 {
                best_match = Some(new_m);
            }
        }

        let best_match = best_match.expect("No supported audio format found");
        let format = best_match.format;
        let channels = format.channels.len();

        let executor = Arc::new(AudioExecutor);
        let event_loop = cpal::EventLoop::new();

        let (mut voice, stream) = cpal::Voice::new(&endpoint, &format, &event_loop).unwrap();

        let (tx, rx) = channel();
        let mut samples = SamplesIterator::new(rx);

        task::spawn(stream.for_each(move |buffer| -> Result<_, ()> {
            match buffer {
                cpal::UnknownTypeBuffer::U16(mut buffer) => {
                    for (sample, value) in buffer.chunks_mut(channels).
                        zip(&mut samples) {
                        let value = ((value as i32) + ::std::i16::MAX as i32) as u16;
                        for out in sample.iter_mut() { *out = value; }
                    }
                },

                cpal::UnknownTypeBuffer::I16(mut buffer) => {
                    for (sample, value) in buffer.chunks_mut(channels).
                        zip(&mut samples) {
                        for out in sample.iter_mut() { *out = value; }
                    }
                },

                cpal::UnknownTypeBuffer::F32(mut buffer) => {
                    for (sample, value) in buffer.chunks_mut(channels).
                        zip(&mut samples) {
                        let value = (value as f32) / ::std::i16::MAX as f32;
                        for out in sample.iter_mut() { *out = value; }
                    }
                },
            }

            Ok(())
        })).execute(executor);

        voice.play();

        thread::spawn(move || { event_loop.run() });

        Audio {
            endpoint: endpoint,
            format: format,
            voice: voice,
            tx: tx,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        let cpal::SamplesRate(rate) = self.format.samples_rate;
        rate
    }

    pub fn add_samples(&mut self, samples: Vec<i16>) {
        let _ = self.tx.send(samples).unwrap();
    }
}

use std::collections::VecDeque;

struct SamplesIterator<T> {
    rx: Receiver<Vec<T>>, 
    buf: VecDeque<T>,
}

impl<T> SamplesIterator<T> {
    fn new(rx: Receiver<Vec<T>>) -> SamplesIterator<T> {
        SamplesIterator {
            rx: rx,
            buf: VecDeque::new(),
        }
    }
}

impl<T> Iterator for SamplesIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.buf.len() == 0 {
            match self.rx.try_recv() {
                Ok(r) => {
                    let mut vec = r.into_iter().collect();
                    self.buf.append(&mut vec);
                    self.buf.pop_front()
                },
                Err(_) => None
            }
        } else {
            self.buf.pop_front() 
        }
    }
}
