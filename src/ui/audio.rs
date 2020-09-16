use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn add_samples(&mut self, samples: Vec<i16>);
    fn close(&mut self);
}

pub struct Null;
impl Audio for Null {
    fn sample_rate(&self) -> u32 {
        48000
    }
    fn add_samples(&mut self, _samples: Vec<i16>) {}
    fn close(&mut self) {}
}

pub struct CpalAudio {
    host: cpal::Host,
    device: cpal::Device,
    stream: cpal::Stream,
    format: cpal::StreamConfig,
    tx: Sender<Vec<i16>>,
}

pub struct CpalSync {
    parker: crossbeam::sync::Parker,
}

impl super::sync::FrameSync for CpalSync {
    fn sync_frame(&mut self) {
        self.parker.park()
    }
}

impl CpalAudio {
    pub fn new(refresh_rate: f64) -> Option<(CpalAudio, CpalSync)> {
        let allowed_sample_rates = vec![
            cpal::SampleRate(48000),
            cpal::SampleRate(44100),
            cpal::SampleRate(96000),
        ];
        let host = cpal::default_host();
        let device = host.default_output_device()?;

        struct Match {
            sample_rate: cpal::SampleRate,
            sample_rate_idx: usize,
            channels: u16,
            data_type: cpal::SampleFormat,
        }

        let mut best_match: Option<Match> = None;
        let formats = device.supported_output_configs().ok()?;
        for f in formats {
            let sample_rate_idx = allowed_sample_rates
                .iter()
                .position(|x| f.min_sample_rate() <= *x && f.max_sample_rate() >= *x);

            // Must be one of our requested sample rates
            if sample_rate_idx.is_none() {
                continue;
            }
            let sample_rate_idx = sample_rate_idx.unwrap();
            let sample_rate = allowed_sample_rates[sample_rate_idx];

            let channels = f.channels();
            // Need atleast one channel
            if channels == 0 {
                continue;
            }

            let data_type = f.sample_format();
            // Only supporting I16 samples for now
            if data_type != cpal::SampleFormat::I16 {
                continue;
            }

            let new_match = Match {
                sample_rate,
                sample_rate_idx,
                channels,
                data_type,
            };

            if let Some(current_match) = best_match.as_ref() {
                if sample_rate_idx < current_match.sample_rate_idx {
                    // Prefer our first choice for sample rate
                    best_match = Some(new_match);
                    continue;
                } else if sample_rate_idx < current_match.sample_rate_idx {
                    continue;
                } else if channels < current_match.channels {
                    // Prefer fewer channels
                    best_match = Some(new_match);
                    continue;
                } else {
                    continue;
                }
            } else {
                best_match = Some(new_match);
            }
        }

        let best_match = best_match?;
        println!(
            "Audio Format: {} channel(s) {} sample rate",
            best_match.channels, best_match.sample_rate.0
        );
        let format = cpal::StreamConfig {
            channels: best_match.channels,
            sample_rate: best_match.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(1024),
        };
        let channels = format.channels as usize;
        let sample_rate = format.sample_rate.0;

        let (tx, rx) = channel();
        let mut samples = SamplesIterator::new(rx);

        let parker = crossbeam::sync::Parker::new();
        let unparker = parker.unparker().clone();

        let stream = device
            .build_output_stream(
                &format,
                move |buffer: &mut [i16], _callback_info: &cpal::OutputCallbackInfo| {
                    for (sample, value) in buffer.chunks_mut(channels).zip(&mut samples) {
                        for out in sample.iter_mut() {
                            *out = value;
                        }
                    }
                    if samples.buf.len() < (sample_rate as f64 / refresh_rate) as usize {
                        unparker.unpark();
                    }
                },
                |err| eprintln!("{:?}", err),
            )
            .ok()?;
        stream.play().ok()?;

        Some((
            CpalAudio {
                host,
                device,
                stream,
                format,
                tx,
            },
            CpalSync { parker },
        ))
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

    fn close(&mut self) {}
}

struct SamplesIterator<T> {
    rx: Receiver<Vec<T>>,
    buf: VecDeque<T>,
}

impl<T> SamplesIterator<T> {
    fn new(rx: Receiver<Vec<T>>) -> SamplesIterator<T> {
        SamplesIterator {
            rx,
            buf: VecDeque::new(),
        }
    }
}

impl<T> Iterator for SamplesIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(r) => {
                self.buf.reserve(r.len());
                self.buf.extend(r.into_iter());
            }
            Err(_) => (),
        }
        self.buf.pop_front()
    }
}
