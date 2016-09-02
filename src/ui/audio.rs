extern crate cpal;
extern crate futures;

use self::futures::stream::Stream;
use self::futures::Future;

use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};

pub struct Audio {
    endpoint: cpal::Endpoint,
    format: cpal::Format,
    voice: cpal::Voice,
    tx: Sender<Vec<i16>>,
}

impl Audio {
    pub fn new(sample_rate: u32) -> Audio {
        let endpoint = cpal::get_default_endpoint().unwrap();
        let format = endpoint.get_supported_formats_list().unwrap().filter(|x| {
            x.samples_rate == cpal::SamplesRate(sample_rate) &&
            x.data_type == cpal::SampleFormat::I16 &&
            x.channels.len() == 1}).next().unwrap();
        
        let channels = format.channels.len();

        let event_loop = cpal::EventLoop::new();

        let (mut voice, stream) = cpal::Voice::new(&endpoint, &format, &event_loop).unwrap();

        let (tx, rx) = channel();
        let mut samples = SamplesIterator::new(rx);

        stream.for_each(move |buffer| -> Result<_, ()> {
            match buffer {
                cpal::UnknownTypeBuffer::U16(mut buffer) => {
                    println!("u16");
                },

                cpal::UnknownTypeBuffer::I16(mut buffer) => {
                    for (sample, value) in buffer.chunks_mut(channels).
                        zip(&mut samples) {
                        for out in sample.iter_mut() { *out = value; }
                    }
                },

                cpal::UnknownTypeBuffer::F32(mut buffer) => {
                    println!("float");
                },
            }

            Ok(())
        }).forget();

        voice.play();

        thread::spawn(move || { event_loop.run() });

        Audio {
            endpoint: endpoint,
            format: format,
            voice: voice,
            tx: tx,
        }
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
