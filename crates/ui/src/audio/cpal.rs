use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, OutputCallbackInfo, Sample};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use super::{Audio, SamplesReceiver, SamplesSender, samples_channel};

// Number of samples per audio frame
const DEVICE_BUFFER: u32 = 128;

// Number of audio frames to buffer
const DEVICE_BUFFER_DEPTH: usize = 2;

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
    pub fn new() -> Result<(CpalAudio, SamplesSender), Error> {
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
            buffer_size: cpal::SupportedBufferSize,
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
                buffer_size: *f.buffer_size(),
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
        let buffer_len = match best_match.buffer_size {
            cpal::SupportedBufferSize::Range { min, max }
                if min < DEVICE_BUFFER && max >= DEVICE_BUFFER =>
            {
                DEVICE_BUFFER
            }
            cpal::SupportedBufferSize::Range { max, .. } if max < DEVICE_BUFFER => max,
            cpal::SupportedBufferSize::Range { min, .. } => min,
            cpal::SupportedBufferSize::Unknown => DEVICE_BUFFER,
        };

        let total_buffer_len = buffer_len as usize * DEVICE_BUFFER_DEPTH;

        let range_desc = match best_match.buffer_size {
            cpal::SupportedBufferSize::Range { min, max } => &format!("[{min}, {max}]"),
            cpal::SupportedBufferSize::Unknown => "unknown",
        };

        tracing::debug!(
            "{:?}: {} channel(s), {} sample rate, {} format, range: {}, {} buffer samples, {}ms buffer duration",
            host.id(),
            best_match.channels,
            best_match.sample_rate.0,
            best_match.data_type,
            range_desc,
            total_buffer_len,
            std::time::Duration::from_secs_f64(
                total_buffer_len as f64 / best_match.sample_rate.0 as f64
            )
            .as_millis()
        );

        let format = cpal::StreamConfig {
            channels: best_match.channels,
            sample_rate: best_match.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(buffer_len as u32),
        };
        let channels = format.channels as usize;

        let (samples_tx, samples_rx) = samples_channel(
            format.sample_rate.0 as usize,
            buffer_len as usize,
            DEVICE_BUFFER_DEPTH,
        );

        let host_id = host.id();
        let volume = Arc::new(AtomicU32::new(u32::MAX));
        let err_handler = move |err| tracing::error!("{:?}: {:?}", host_id, err);

        let stream = match best_match.data_type {
            cpal::SampleFormat::I16 => device.build_output_stream(
                &format,
                output_callback::<i16>(samples_rx, channels, volume.clone()),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &format,
                output_callback::<u16>(samples_rx, channels, volume.clone()),
                err_handler,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_output_stream(
                &format,
                output_callback::<f32>(samples_rx, channels, volume.clone()),
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
            },
            samples_tx,
        ))
    }
}

fn output_callback<S: Sample + FromSample<i16>>(
    mut sample_source: SamplesReceiver<i16>,
    channels: usize,
    volume: Arc<AtomicU32>,
) -> impl FnMut(&mut [S], &OutputCallbackInfo) + Send + 'static {
    move |buffer: &mut [S], _| {
        let volume = volume.load(Ordering::Relaxed) as f64 / u32::MAX as f64;
        sample_source.grow_buffer_len(buffer.len());

        for (sample, value) in buffer.chunks_mut(channels).zip(&mut sample_source) {
            let value = (value as f64 * volume) as i16;
            let s = value.to_sample();
            for out in sample.iter_mut() {
                *out = s;
            }
        }

        sample_source.notify();
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
