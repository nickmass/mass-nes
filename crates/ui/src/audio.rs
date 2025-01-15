use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, Sample};
use crossbeam::sync::{Parker, Unparker};

use direct_ring_buffer::{create_ring_buffer, Consumer, Producer};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
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

// Number of frames to store samples for across all buffers
#[cfg(not(target_arch = "wasm32"))]
const BUFFER_FRAMES: f64 = 1.0;

#[cfg(target_arch = "wasm32")]
const BUFFER_FRAMES: f64 = 2.5;

// Number of samples stored directly in the audio device buffers
#[cfg(not(target_arch = "wasm32"))]
const DEVICE_BUFFER: usize = 64;

#[cfg(target_arch = "wasm32")]
const DEVICE_BUFFER: usize = 256;

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

pub struct CpalAudio {
    _host: cpal::Host,
    _device: cpal::Device,
    stream: cpal::Stream,
    format: cpal::StreamConfig,
    volume: Arc<AtomicU32>,
}

impl CpalAudio {
    pub fn new(refresh_rate: f64) -> Result<(CpalAudio, SamplesSync, SamplesProducer), Error> {
        let device_buffer = DEVICE_BUFFER;
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
        let samples_sync = SamplesSync::new(samples_min_len);
        let samples = SamplesIterator::new(rx, samples_sync.tracker());

        let host_id = host.id();
        let volume = Arc::new(AtomicU32::new(u32::MAX));
        let err_handler = move |err| tracing::error!("{:?}: {:?}", host_id, err);

        let stream = match best_match.data_type {
            cpal::SampleFormat::I16 => device.build_output_stream(
                &format,
                output_callback::<i16>(samples, channels, volume.clone()),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &format,
                output_callback::<u16>(samples, channels, volume.clone()),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_output_stream(
                &format,
                output_callback::<f32>(samples, channels, volume.clone()),
                err_handler,
                None,
            )?,
            _ => return Err(Error::NoMatchingOutputConfig),
        };
        let producer = SamplesProducer {
            tx,
            sync: samples_sync.tracker(),
        };

        Ok((
            CpalAudio {
                _host: host,
                _device: device,
                stream,
                format,
                volume,
            },
            samples_sync,
            producer,
        ))
    }
}

fn output_callback<S: Sample + FromSample<i16>>(
    mut sample_source: SamplesIterator<i16>,
    channels: usize,
    volume: Arc<AtomicU32>,
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
    }
}

impl Audio for CpalAudio {
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

struct SamplesIterator<T> {
    rx: Consumer<T>,
    buf: VecDeque<T>,
    pending_req: bool,
    last_sample: T,
    sync: SamplesTracker,
}

impl<T: Default + Copy> SamplesIterator<T> {
    fn new(rx: Consumer<T>, sync: SamplesTracker) -> SamplesIterator<T> {
        SamplesIterator {
            rx,
            buf: VecDeque::new(),
            pending_req: false,
            last_sample: T::default(),
            sync,
        }
    }
}

pub struct SamplesProducer {
    tx: Producer<i16>,
    sync: SamplesTracker,
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
        self.sync.set_avail(self.tx.available());
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
        self.sync.set_avail(self.buf.len());

        if let Some(sample) = self.buf.pop_front() {
            self.last_sample = sample;
            Some(sample)
        } else {
            Some(self.last_sample)
        }
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

#[derive(Debug)]
pub struct SamplesSync {
    min: usize,
    avail: Arc<AtomicUsize>,
    parker: Parker,
}

impl SamplesSync {
    pub fn new(min: usize) -> Self {
        let parker = Parker::new();
        Self {
            min,
            avail: Arc::new(AtomicUsize::new(0)),
            parker,
        }
    }

    fn tracker(&self) -> SamplesTracker {
        SamplesTracker {
            avail: self.avail.clone(),
            unparker: self.parker.unparker().clone(),
        }
    }

    pub fn need_samples(&self) -> bool {
        let avail = self.avail.load(Ordering::Relaxed);
        self.min > avail
    }

    pub fn wait_for_need_samples(&self) {
        loop {
            if self.need_samples() {
                break;
            }
            self.parker.park();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SamplesTracker {
    avail: Arc<AtomicUsize>,
    unparker: Unparker,
}

impl SamplesTracker {
    fn set_avail(&self, avail: usize) {
        self.avail.store(avail, Ordering::Relaxed);
        self.unpark();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn unpark(&self) {
        self.unparker.unpark();
    }

    #[cfg(target_arch = "wasm32")]
    fn unpark(&self) {}
}
