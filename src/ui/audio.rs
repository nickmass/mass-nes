extern crate cpal;

use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn add_samples(&mut self, samples: Vec<i16>);
    fn close(self);
}

pub struct CpalAudio {
    device: cpal::Device,
    format: cpal::Format,
    tx: Sender<Vec<i16>>,
}

impl CpalAudio {
    pub fn new() -> CpalAudio {
        let allowed_sample_rates = vec![cpal::SampleRate(48000), cpal::SampleRate(44100), cpal::SampleRate(96000)];
        let device = cpal::default_output_device().expect("No audio output avaliable");

        struct Match {
            rate: cpal::SampleRate,
            sr: usize,
            chan: u16,
            data_type: cpal::SampleFormat,
        }

        let mut best_match = None;
        let formats = device.supported_output_formats().expect("No audio formats found");
        for f in formats {
            let s = allowed_sample_rates.iter().position(|x| f.min_sample_rate <= *x && f.max_sample_rate >= *x);
            let c = f.channels;
            let d = f.data_type;
            if s.is_none() { continue; }
            let s = s.unwrap();
            let new_m = Match {rate: allowed_sample_rates[s], sr: s, chan: c, data_type: d };
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
        let format = cpal::Format {
            channels: best_match.chan,
            sample_rate: best_match.rate,
            data_type: best_match.data_type,
        };
        let channels = format.channels as usize;

        let event_loop = cpal::EventLoop::new();

        let stream = event_loop.build_output_stream(&device, &format).expect("Could not build output stream");
        event_loop.play_stream(stream);

        let (tx, rx) = channel();
        let mut samples = SamplesIterator::new(rx);

        thread::spawn(move || { event_loop.run(move |_stream_id, stream_data| {
            if let cpal::StreamData::Output { buffer } = stream_data {
                match buffer {
                    cpal::UnknownTypeOutputBuffer::U16(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels).
                            zip(&mut samples) {
                                let value = ((value as i32) + ::std::i16::MAX as i32) as u16;
                                for out in sample.iter_mut() { *out = value; }
                            }
                    },

                    cpal::UnknownTypeOutputBuffer::I16(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels).
                            zip(&mut samples) {
                                for out in sample.iter_mut() { *out = value; }
                            }
                    },

                    cpal::UnknownTypeOutputBuffer::F32(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels).
                            zip(&mut samples) {
                                let value = (value as f32) / ::std::i16::MAX as f32;
                                for out in sample.iter_mut() { *out = value; }
                            }
                    },
                }
            }
        })
        });

        CpalAudio {
            device,
            format,
            tx,
        }
    }

}

impl Audio for CpalAudio {
    fn sample_rate(&self) -> u32 {
        let cpal::SampleRate(rate) = self.format.sample_rate;
        rate
    }

    fn add_samples(&mut self, samples: Vec<i16>) {
        let _ = self.tx.send(samples).unwrap();
    }

    fn close(self) {

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

extern crate rodio;

pub struct RodioAudio {
    sample_rate: u32,
    endpoint: rodio::Endpoint,
    sink: rodio::Sink,
}

impl RodioAudio {
    pub fn new(sample_rate: u32) -> RodioAudio {
        let endpoint = rodio::default_endpoint().expect("Could not get default audio endpoint");
        let sink = rodio::Sink::new(&endpoint);
        RodioAudio {
            sample_rate: sample_rate,
            endpoint: endpoint,
            sink: sink,
        }
    }
}

impl Audio for RodioAudio {
    fn sample_rate(&self) -> u32 { self.sample_rate }

    fn add_samples(&mut self, samples: Vec<i16>) {
        let source = RodioSamples {
            sample_rate: self.sample_rate,
            samples: samples,
            position: 0,
        };
        self.sink.append(source);
    }

    fn close(self) {
    }
}

struct RodioSamples {
    sample_rate: u32,
    samples: Vec<i16>,
    position: usize,
}

impl Iterator for RodioSamples {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let val = self.samples.get(self.position);
        self.position += 1;
        val.map(|x| *x)
    }
}

impl rodio::Source for RodioSamples {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> u16 { 1 }

    fn samples_rate(&self) -> u32 { self.sample_rate }

    fn total_duration(&self) -> Option<::std::time::Duration> {
        None
    }
}
