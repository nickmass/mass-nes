use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, Sample};

use direct_ring_buffer::{create_ring_buffer, Consumer, Producer};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn play(&mut self);
    fn pause(&mut self);
    fn volume(&mut self, volume: f32);
}

pub struct Null;

impl Audio for Null {
    fn sample_rate(&self) -> u32 {
        48000
    }

    fn play(&mut self) {}
    fn pause(&mut self) {}
    fn volume(&mut self, _volume: f32) {}
}

const BUFFER_FRAMES: f64 = 2.0;

#[derive(Debug)]
pub enum Error {
    NoDefaultOutputDevice,
    NoMatchingOutputConfig,
    SupportedStreamConfigsError(Box<cpal::SupportedStreamConfigsError>),
    BuildStreamError(Box<cpal::BuildStreamError>),
    PlayStreamError(Box<cpal::PlayStreamError>),
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
        Self::SupportedStreamConfigsError(Box::new(value))
    }
}

impl From<cpal::BuildStreamError> for Error {
    fn from(value: cpal::BuildStreamError) -> Self {
        Self::BuildStreamError(Box::new(value))
    }
}

impl From<cpal::PlayStreamError> for Error {
    fn from(value: cpal::PlayStreamError) -> Self {
        Self::PlayStreamError(Box::new(value))
    }
}

pub struct CpalAudio<P: Parker> {
    _host: cpal::Host,
    _device: cpal::Device,
    stream: cpal::Stream,
    format: cpal::StreamConfig,
    volume: Arc<AtomicU32>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: Parker> CpalAudio<P> {
    pub fn new(
        parker: P,
        refresh_rate: f64,
        device_buffer: usize,
    ) -> Result<(CpalAudio<P>, P, SamplesProducer), Error> {
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
        let samples_min_len = (((best_match.sample_rate.0 as f64 / refresh_rate) * BUFFER_FRAMES)
            .ceil() as usize)
            .saturating_sub(device_buffer);

        let samples_min_len = samples_min_len.max(device_buffer);

        tracing::debug!(
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
                device_buffer as u32 * best_match.data_type.sample_size() as u32,
            ),
        };
        let channels = format.channels as usize;

        let (tx, rx) = create_ring_buffer(best_match.sample_rate.0 as usize);
        let samples = SamplesIterator::new(rx);

        let unparker = parker.unparker();

        let host_id = host.id();
        let volume = Arc::new(AtomicU32::new(u32::MAX));
        let err_handler = move |err| tracing::error!("{:?}: {:?}", host_id, err);

        let stream = match best_match.data_type {
            cpal::SampleFormat::I16 => device.build_output_stream(
                &format,
                output_callback::<i16, _>(
                    samples,
                    samples_min_len,
                    channels,
                    volume.clone(),
                    unparker,
                ),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &format,
                output_callback::<u16, _>(
                    samples,
                    samples_min_len,
                    channels,
                    volume.clone(),
                    unparker,
                ),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_output_stream(
                &format,
                output_callback::<f32, _>(
                    samples,
                    samples_min_len,
                    channels,
                    volume.clone(),
                    unparker,
                ),
                err_handler,
                None,
            )?,
            _ => return Err(Error::NoMatchingOutputConfig),
        };

        Ok((
            CpalAudio {
                _host: host,
                _device: device,
                stream,
                format,
                volume,
                _marker: Default::default(),
            },
            parker,
            SamplesProducer { tx },
        ))
    }
}

fn output_callback<S: Sample + FromSample<i16>, U: Unparker>(
    mut sample_source: SamplesIterator<i16>,
    samples_min_len: usize,
    channels: usize,
    volume: Arc<AtomicU32>,
    mut unparker: U,
) -> impl FnMut(&mut [S], &OutputCallbackInfo) + Send + 'static {
    move |buffer: &mut [S], _| {
        let volume = volume.load(Ordering::Relaxed) as f64 / u32::MAX as f64;

        for (sample, value) in buffer.chunks_mut(channels).zip(&mut sample_source) {
            let value = (value as f64 * volume) as i16;
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

impl<P: Parker> Audio for CpalAudio<P> {
    fn sample_rate(&self) -> u32 {
        let cpal::SampleRate(rate) = self.format.sample_rate;
        rate
    }

    fn play(&mut self) {
        let _ = self.stream.play();
    }

    fn pause(&mut self) {
        let _ = self.stream.pause();
    }

    fn volume(&mut self, volume: f32) {
        let volume = volume.max(0.0).min(1.0);
        let new_volume = if volume == 1.0 {
            u32::MAX
        } else if volume == 0.0 {
            0
        } else {
            (u32::MAX as f32 * volume) as u32
        };

        self.volume.store(new_volume, Ordering::Relaxed);
    }
}

pub trait Parker {
    type Unparker: Unparker;

    fn unparker(&self) -> Self::Unparker;
}

pub trait Unparker: Send + 'static {
    fn unpark(&mut self);
}

struct SamplesIterator<T> {
    rx: Consumer<T>,
    buf: VecDeque<T>,
    pending_req: bool,
    last_sample: T,
}

impl<T: Default + Copy> SamplesIterator<T> {
    fn new(rx: Consumer<T>) -> SamplesIterator<T> {
        SamplesIterator {
            rx,
            buf: VecDeque::new(),
            pending_req: false,
            last_sample: T::default(),
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

pub struct SamplesProducer {
    tx: Producer<i16>,
}

impl SamplesProducer {
    pub fn add_samples(&mut self, samples: &[i16]) {
        let mut total_count = 0;
        self.tx.write_slices(
            |buf, _offset| {
                let to_write = (samples.len() - total_count).min(buf.len());
                buf[..to_write].copy_from_slice(&samples[total_count..total_count + to_write]);
                total_count += to_write;
                to_write
            },
            Some(samples.len()),
        );
    }
}

impl<T: Copy> Iterator for SamplesIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.rx.read_slices(
            |samples, _offset| {
                self.buf.extend(samples);
                self.pending_req = false;
                samples.len()
            },
            None,
        );

        if let Some(sample) = self.buf.pop_front() {
            self.last_sample = sample;
            Some(sample)
        } else {
            Some(self.last_sample)
        }
    }
}

pub enum AudioDevices<P: Parker> {
    Cpal(CpalAudio<P>),
    Null(Null),
}

impl<P: Parker> Audio for AudioDevices<P> {
    fn sample_rate(&self) -> u32 {
        match self {
            AudioDevices::Cpal(a) => a.sample_rate(),
            AudioDevices::Null(a) => a.sample_rate(),
        }
    }

    fn play(&mut self) {
        match self {
            AudioDevices::Cpal(a) => a.play(),
            AudioDevices::Null(a) => a.play(),
        }
    }

    fn pause(&mut self) {
        match self {
            AudioDevices::Cpal(a) => a.pause(),
            AudioDevices::Null(a) => a.pause(),
        }
    }

    fn volume(&mut self, volume: f32) {
        match self {
            AudioDevices::Cpal(a) => a.volume(volume),
            AudioDevices::Null(a) => a.volume(volume),
        }
    }
}

impl<P: Parker> From<CpalAudio<P>> for AudioDevices<P> {
    fn from(value: CpalAudio<P>) -> Self {
        AudioDevices::Cpal(value)
    }
}

impl<P: Parker> From<Null> for AudioDevices<P> {
    fn from(value: Null) -> Self {
        AudioDevices::Null(value)
    }
}
