use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, Sample};
use crossbeam::sync::{Parker, Unparker};

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn add_samples(&mut self, samples: Vec<i16>);
}

pub struct Null;
impl Audio for Null {
    fn sample_rate(&self) -> u32 {
        48000
    }
    fn add_samples(&mut self, _samples: Vec<i16>) {}
}

pub struct CpalAudio {
    _host: cpal::Host,
    _device: cpal::Device,
    _stream: cpal::Stream,
    format: cpal::StreamConfig,
    tx: Sender<Vec<i16>>,
}

const BUFFER_FRAMES: f64 = 1.0;

#[derive(Debug)]
pub enum Error {
    NoDefaultOutputDevice,
    NoMatchingOutputConfig,
    SupportedStreamConfigsError(cpal::SupportedStreamConfigsError),
    BuildStreamError(cpal::BuildStreamError),
    PlayStreamError(cpal::PlayStreamError),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoDefaultOutputDevice => write!(f, "no default audio output device"),
            Error::NoMatchingOutputConfig => write!(f, "no valid output device format"),
            Error::SupportedStreamConfigsError(e) => e.fmt(f),
            Error::BuildStreamError(e) => e.fmt(f),
            Error::PlayStreamError(e) => e.fmt(f),
        }
    }
}

impl From<cpal::SupportedStreamConfigsError> for Error {
    fn from(value: cpal::SupportedStreamConfigsError) -> Self {
        Self::SupportedStreamConfigsError(value)
    }
}

impl From<cpal::BuildStreamError> for Error {
    fn from(value: cpal::BuildStreamError) -> Self {
        Self::BuildStreamError(value)
    }
}

impl From<cpal::PlayStreamError> for Error {
    fn from(value: cpal::PlayStreamError) -> Self {
        Self::PlayStreamError(value)
    }
}

impl CpalAudio {
    pub fn new(refresh_rate: f64) -> Result<(CpalAudio, CpalSync), Error> {
        let allowed_sample_rates = [
            cpal::SampleRate(48000),
            cpal::SampleRate(44100),
            cpal::SampleRate(96000),
        ];
        let host = cpal::platform::default_host();
        let device = host
            .default_output_device()
            .ok_or(Error::NoDefaultOutputDevice)?;

        struct Match {
            sample_rate: cpal::SampleRate,
            sample_rate_idx: usize,
            channels: u16,
            data_type: cpal::SampleFormat,
        }

        let mut best_match: Option<Match> = None;
        let formats = device.supported_output_configs()?;
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

            let data_type = match f.sample_format() {
                f @ cpal::SampleFormat::I16 => f,
                f @ cpal::SampleFormat::U16 => f,
                f @ cpal::SampleFormat::F32 => f,
                _ => continue,
            };

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
                } else if channels < current_match.channels {
                    // Prefer fewer channels
                    best_match = Some(new_match);
                    continue;
                } else if data_type == cpal::SampleFormat::I16
                    && current_match.data_type != cpal::SampleFormat::I16
                {
                    // Prefer i16
                    best_match = Some(new_match);
                    continue;
                } else {
                    continue;
                }
            } else {
                best_match = Some(new_match);
            }
        }

        let best_match = best_match.ok_or(Error::NoMatchingOutputConfig)?;
        let buffer_size = 64;
        let samples_min_len = (((best_match.sample_rate.0 as f64 / refresh_rate) * BUFFER_FRAMES)
            .ceil() as usize)
            .saturating_sub(buffer_size as usize);

        let samples_min_len = samples_min_len.max(buffer_size as usize);

        eprintln!(
            "{:?}: {} channel(s), {} sample rate, {} format, {} buffer samples, {}ms buffer duration",
            host.id(),
            best_match.channels,
            best_match.sample_rate.0,
            best_match.data_type,
            samples_min_len,
            std::time::Duration::from_secs_f64(
                samples_min_len as f64 / best_match.sample_rate.0 as f64
            )
            .as_millis()
        );

        let format = cpal::StreamConfig {
            channels: best_match.channels,
            sample_rate: best_match.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(
                buffer_size * best_match.data_type.sample_size() as u32,
            ),
        };
        let channels = format.channels as usize;

        let (tx, rx) = channel();
        let samples = SamplesIterator::new(rx);

        let sync = CpalSync::new();
        let unparker = sync.unparker();

        let host_id = host.id();
        let err_handler = move |err| eprintln!("{:?}: {:?}", host_id, err);

        let stream = match best_match.data_type {
            cpal::SampleFormat::I16 => device.build_output_stream(
                &format,
                output_callback::<i16>(samples, samples_min_len, channels, unparker),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &format,
                output_callback::<u16>(samples, samples_min_len, channels, unparker),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_output_stream(
                &format,
                output_callback::<f32>(samples, samples_min_len, channels, unparker),
                err_handler,
                None,
            )?,
            _ => return Err(Error::NoMatchingOutputConfig),
        };

        stream.play()?;

        Ok((
            CpalAudio {
                _host: host,
                _device: device,
                _stream: stream,
                format,
                tx,
            },
            sync,
        ))
    }
}

fn output_callback<S: Sample + FromSample<i16>>(
    mut sample_source: SamplesIterator<i16>,
    samples_min_len: usize,
    channels: usize,
    unparker: Unparker,
) -> impl FnMut(&mut [S], &OutputCallbackInfo) + Send + 'static {
    move |buffer: &mut [S], _| {
        for (sample, value) in buffer
            .chunks_mut(channels)
            .zip((&mut sample_source).chain(std::iter::repeat(i16::EQUILIBRIUM)))
        {
            let s = value.to_sample();
            for out in sample.iter_mut() {
                *out = s;
            }
        }

        if sample_source.len() < samples_min_len {
            if !sample_source.pending_request() {
                sample_source.request_samples();
                unparker.unpark();
            }
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
}

pub struct CpalSync {
    parker: Parker,
}

impl CpalSync {
    fn new() -> Self {
        Self {
            parker: Parker::new(),
        }
    }

    fn unparker(&self) -> Unparker {
        self.parker.unparker().clone()
    }
}

impl super::sync::FrameSync for CpalSync {
    fn sync_frame(&mut self) {
        self.parker.park()
    }
}

struct SamplesIterator<T> {
    rx: Receiver<Vec<T>>,
    buf: VecDeque<T>,
    pending_req: bool,
}

impl<T> SamplesIterator<T> {
    fn new(rx: Receiver<Vec<T>>) -> SamplesIterator<T> {
        SamplesIterator {
            rx,
            buf: VecDeque::new(),
            pending_req: false,
        }
    }

    fn request_samples(&mut self) {
        self.pending_req = true;
    }

    fn pending_request(&self) -> bool {
        self.pending_req
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}

impl<T> Iterator for SamplesIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.rx.try_recv() {
            Ok(r) => {
                self.pending_req = false;
                self.buf.extend(r);
            }
            Err(_) => (),
        }
        self.buf.pop_front()
    }
}

pub enum AudioDevices {
    Cpal(CpalAudio),
    Null(Null),
}

impl Audio for AudioDevices {
    fn sample_rate(&self) -> u32 {
        match self {
            AudioDevices::Cpal(a) => a.sample_rate(),
            AudioDevices::Null(a) => a.sample_rate(),
        }
    }

    fn add_samples(&mut self, samples: Vec<i16>) {
        match self {
            AudioDevices::Cpal(a) => a.add_samples(samples),
            AudioDevices::Null(a) => a.add_samples(samples),
        }
    }
}

impl From<CpalAudio> for AudioDevices {
    fn from(value: CpalAudio) -> Self {
        AudioDevices::Cpal(value)
    }
}

impl From<Null> for AudioDevices {
    fn from(value: Null) -> Self {
        AudioDevices::Null(value)
    }
}
