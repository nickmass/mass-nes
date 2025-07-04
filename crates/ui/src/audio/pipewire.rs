use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    thread::JoinHandle,
};

use super::{Audio, SamplesReceiver, SamplesSender, samples_channel};

const SAMPLE_RATE: u32 = 48000;
const SAMPLE_LATENCY: u32 = 32;

#[derive(Debug)]
pub enum Error {
    InitFailed,
    Pipewire(pipewire::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InitFailed => write!(f, "pipewire init failed"),
            Error::Pipewire(error) => error.fmt(f),
        }
    }
}

impl From<pipewire::Error> for Error {
    fn from(value: pipewire::Error) -> Self {
        Self::Pipewire(value)
    }
}

pub struct PipewireAudio {
    #[allow(unused)]
    main_thread: JoinHandle<()>,
    status: Arc<PlayStatus>,
}

impl PipewireAudio {
    pub fn new() -> Result<(Self, SamplesSender), Error> {
        let sample_rate = SAMPLE_RATE;
        let status = PlayStatus::new(sample_rate);
        let status = Arc::new(status);
        let (samples_tx, samples_rx) =
            samples_channel(sample_rate as usize, SAMPLE_LATENCY as usize, 2);

        let (init_tx, init_rx) = sync_channel(1);

        let main_thread = std::thread::Builder::new()
            .name("pipewire_audio".into())
            .spawn({
                let status = status.clone();
                move || {
                    if let Err(err) = PipewireMainThread::run(&init_tx, status, samples_rx) {
                        init_tx.send(Err(err)).unwrap();
                    };
                }
            })
            .map_err(|_| Error::InitFailed)?;

        let _ = init_rx.recv().map_err(|_| Error::InitFailed)??;

        Ok((
            Self {
                main_thread,
                status,
            },
            samples_tx,
        ))
    }
}

impl Audio for PipewireAudio {
    fn sample_rate(&self) -> u32 {
        self.status.sample_rate()
    }

    fn play(&mut self) {
        self.status.play();
    }

    fn pause(&mut self) {
        self.status.pause();
    }

    fn volume(&mut self, volume: f32) {
        self.status.set_volume(volume);
    }
}

struct PipewireMainThread;

impl PipewireMainThread {
    fn run(
        init: &SyncSender<Result<(), pipewire::Error>>,
        status: Arc<PlayStatus>,
        samples_rx: SamplesReceiver<i16>,
    ) -> Result<(), pipewire::Error> {
        let sample_rate = status.sample_rate();
        tracing::debug!("pipewire sample rate: {sample_rate} latency: {SAMPLE_LATENCY}");
        pipewire::init();
        let main_loop = pipewire::main_loop::MainLoop::new(None)?;
        let context = pipewire::context::Context::new(&main_loop)?;
        let core = context.connect(None)?;

        let node_latency = format!("{SAMPLE_LATENCY}/{sample_rate}");

        let stream = pipewire::stream::Stream::new(
            &core,
            "mass-nes",
            pipewire::properties::properties! {
                *pipewire::keys::MEDIA_TYPE => "Audio",
                *pipewire::keys::MEDIA_ROLE => "Game",
                *pipewire::keys::MEDIA_CATEGORY => "Playback",
                *pipewire::keys::AUDIO_CHANNELS  => "1",
                *pipewire::keys::NODE_LATENCY => node_latency,
            },
        )?;

        let _listener = stream
            .add_local_listener_with_user_data((status, samples_rx))
            .process(|stream, (status, samples)| match stream.dequeue_buffer() {
                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    let data = &mut datas[0];
                    let stride = std::mem::size_of::<f32>();
                    let volume = status.volume();
                    let mut n_frames = 0;

                    if let Some(slice) = data.data() {
                        if status.is_paused() {
                            for out_sample in
                                slice.chunks_exact_mut(stride).take(SAMPLE_LATENCY as usize)
                            {
                                out_sample.copy_from_slice(&0.0f32.to_le_bytes());
                                n_frames += 1;
                            }
                        } else {
                            for (out_sample, sample) in slice
                                .chunks_exact_mut(stride)
                                .zip(&mut *samples)
                                .take(SAMPLE_LATENCY as usize)
                            {
                                let sample = sample as f32 / i16::MAX as f32 * volume;
                                out_sample.copy_from_slice(&sample.to_le_bytes());
                                n_frames += 1;
                            }
                            samples.notify();
                        }
                    }

                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = stride as i32;
                    *chunk.size_mut() = (stride * n_frames) as u32;
                }
                None => tracing::error!("no pipewire buffer"),
            })
            .register()?;

        use pipewire::spa;
        let mut audio_info = spa::param::audio::AudioInfoRaw::new();
        audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
        audio_info.set_rate(sample_rate);
        audio_info.set_channels(1);
        let mut position = [0; spa::param::audio::MAX_CHANNELS];
        position[0] = spa::sys::SPA_AUDIO_CHANNEL_FC;
        audio_info.set_position(position);

        let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &spa::pod::Value::Object(spa::pod::Object {
                type_: spa::sys::SPA_TYPE_OBJECT_Format,
                id: spa::sys::SPA_PARAM_EnumFormat,
                properties: audio_info.into(),
            }),
        )
        .unwrap()
        .0
        .into_inner();

        let mut params = [spa::pod::Pod::from_bytes(&values).unwrap()];

        stream.connect(
            spa::utils::Direction::Output,
            None,
            pipewire::stream::StreamFlags::AUTOCONNECT
                | pipewire::stream::StreamFlags::MAP_BUFFERS
                | pipewire::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )?;

        init.send(Ok(())).unwrap();

        main_loop.run();

        Ok(())
    }
}

struct PlayStatus {
    volume: AtomicU32,
    paused: AtomicBool,
    sample_rate: AtomicU32,
}

impl PlayStatus {
    fn new(sample_rate: u32) -> Self {
        Self {
            volume: AtomicU32::new(u32::MAX),
            paused: AtomicBool::new(false),
            sample_rate: AtomicU32::new(sample_rate),
        }
    }
    fn set_volume(&self, volume: f32) {
        let volume = (volume.max(0.0).min(1.0) * u32::MAX as f32) as u32;
        self.volume.store(volume, Ordering::Relaxed);
    }

    fn volume(&self) -> f32 {
        let vol = self.volume.load(Ordering::Relaxed);

        if vol == 0 {
            0.0
        } else if vol == u32::MAX {
            1.0
        } else {
            (vol as f32) / (u32::MAX as f32)
        }
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::Relaxed)
    }

    fn play(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}
