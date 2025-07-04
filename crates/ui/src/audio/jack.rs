use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use jack::{AudioOut, ClientOptions, Port};

use super::{Audio, SamplesReceiver, SamplesSender, samples_channel};

const SAMPLE_LATENCY: u32 = 64;

pub struct JackAudio {
    #[allow(unused)]
    client: jack::AsyncClient<Notification, Processor>,
    status: Arc<PlayStatus>,
}

impl JackAudio {
    pub fn new() -> Result<(Self, SamplesSender), jack::Error> {
        let (client, _client_status) = jack::Client::new("nes", ClientOptions::default())?;
        let port = client.register_port("out", jack::AudioOut::default())?;
        let port_name = port.name()?;

        let sample_rate = client.sample_rate() as u32;
        tracing::debug!("JACK sample rate: {sample_rate}");
        let status = Arc::new(PlayStatus::new(sample_rate));

        let (samples_tx, samples_rx) =
            samples_channel(sample_rate as usize, SAMPLE_LATENCY as usize, 2);

        let processor = Processor {
            port,
            samples_rx,
            status: status.clone(),
        };

        let notification = Notification {
            status: status.clone(),
        };

        let client = client.activate_async(notification, processor)?;

        let _ = client.as_client().set_buffer_size(SAMPLE_LATENCY);
        let system_ports =
            client
                .as_client()
                .ports(Some("system:playback_.*"), None, jack::PortFlags::empty());

        for system_port in system_ports {
            client
                .as_client()
                .connect_ports_by_name(&port_name, &system_port)?;
        }

        Ok((Self { client, status }, samples_tx))
    }
}

impl Audio for JackAudio {
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

struct Processor {
    port: Port<AudioOut>,
    samples_rx: SamplesReceiver<i16>,
    status: Arc<PlayStatus>,
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        if self.status.is_paused() {
            return jack::Control::Continue;
        }

        let out_buf = self.port.as_mut_slice(process_scope);
        let volume = self.status.volume();

        for (out, s) in out_buf.iter_mut().zip(&mut self.samples_rx) {
            let s = s as f32 / i16::MAX as f32;
            *out = s * volume;
        }

        self.samples_rx.notify();

        jack::Control::Continue
    }

    fn buffer_size(&mut self, _: &jack::Client, size: jack::Frames) -> jack::Control {
        self.samples_rx.set_buffer_len(size as usize);
        jack::Control::Continue
    }
}

struct Notification {
    status: Arc<PlayStatus>,
}

impl jack::NotificationHandler for Notification {
    fn sample_rate(&mut self, _: &jack::Client, srate: jack::Frames) -> jack::Control {
        self.status.set_sample_rate(srate as u32);
        jack::Control::Continue
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

    fn set_sample_rate(&self, rate: u32) {
        self.sample_rate.store(rate, Ordering::Relaxed);
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
