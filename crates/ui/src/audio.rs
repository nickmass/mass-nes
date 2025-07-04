use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use direct_ring_buffer::{Consumer, Producer, create_ring_buffer};

#[cfg(not(target_arch = "wasm32"))]
mod cpal;
#[cfg(not(target_arch = "wasm32"))]
pub use cpal::CpalAudio;
#[cfg(all(not(target_arch = "wasm32"), feature = "jack"))]
mod jack;
#[cfg(all(not(target_arch = "wasm32"), feature = "jack"))]
pub use jack::JackAudio;
#[cfg(all(not(target_arch = "wasm32"), feature = "pipewire"))]
mod pipewire;
#[cfg(all(not(target_arch = "wasm32"), feature = "pipewire"))]
pub use pipewire::PipewireAudio;

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserAudio;

mod null;
pub use null::Null;

use crate::wav_writer::WavWriter;

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn play(&mut self);
    fn pause(&mut self);
    fn volume(&mut self, volume: f32);
}

fn samples_channel(
    capacity: usize,
    buffer_len: usize,
    buffer_depth: usize,
) -> (SamplesSender, SamplesReceiver<i16>) {
    let (tx, rx) = create_ring_buffer(capacity);

    let buffer_len = Arc::new(AtomicUsize::new(buffer_len));
    let notify = SamplesNotify::new();

    let rx = SamplesReceiver {
        rx,
        buffer_len: buffer_len.clone(),
        last_sample: 0,
        notify: notify.clone(),
    };

    let tx = SamplesSender {
        tx,
        capacity,
        buffer_len,
        notify,
        buffer_depth,
        wav_writer: None,
    };

    (tx, rx)
}

struct SamplesReceiver<T> {
    rx: Consumer<T>,
    buffer_len: Arc<AtomicUsize>,
    last_sample: T,
    notify: SamplesNotify,
}

impl<T> SamplesReceiver<T> {
    fn grow_buffer_len(&self, new_len: usize) {
        let old_len = self.buffer_len.load(Ordering::Relaxed);

        if old_len < new_len {
            self.set_buffer_len(new_len);
        }
    }

    fn set_buffer_len(&self, new_len: usize) {
        let old_len = self.buffer_len.load(Ordering::Relaxed);
        if old_len != new_len {
            #[cfg(not(target_arch = "wasm32"))]
            tracing::debug!("audio buffer resized old size: {old_len}, new size: {new_len}");
            self.buffer_len.store(new_len, Ordering::Relaxed);
        }
    }

    fn notify(&self) {
        self.notify.notify();
    }
}

impl<T: Copy> Iterator for SamplesReceiver<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if let Some(sample) = self.rx.read_element() {
            self.last_sample = sample;
            Some(sample)
        } else {
            Some(self.last_sample)
        }
    }
}

pub struct SamplesSender {
    tx: Producer<i16>,
    capacity: usize,
    buffer_len: Arc<AtomicUsize>,
    buffer_depth: usize,
    notify: SamplesNotify,
    wav_writer: Option<WavWriter>,
}

impl SamplesSender {
    pub fn start_recording(
        &mut self,
        file: std::fs::File,
        sample_rate: u32,
    ) -> std::io::Result<()> {
        self.end_recording()?;

        let wav = WavWriter::new(file, sample_rate)?;

        self.wav_writer = Some(wav);
        Ok(())
    }

    pub fn end_recording(&mut self) -> std::io::Result<()> {
        let Some(wav) = self.wav_writer.take() else {
            return Ok(());
        };

        wav.finalize()
    }

    pub fn add_samples(&mut self, samples: &[i16]) {
        let mut total_count = 0;
        self.tx.write_slices(
            |buf, _offset| {
                let to_write = (samples.len() - total_count).min(buf.len());
                process_samples(self.wav_writer.as_mut(), &buf[..to_write]);
                total_count += to_write;
                to_write
            },
            Some(samples.len()),
        );
    }

    pub fn add_samples_from_blip(&mut self, blip: &mut blip_buf_rs::Blip) {
        let avail = blip.samples_avail() as usize;
        let written = self.tx.write_slices(
            |buf, _offset| {
                let count = blip.read_samples(buf, buf.len() as u32, false) as usize;
                process_samples(self.wav_writer.as_mut(), &buf[..count]);
                count
            },
            Some(avail),
        );

        if written < avail {
            tracing::warn!("run out of sample space");
            blip.clear();
        }
    }

    pub fn wants_samples(&self) -> Option<usize> {
        let used = self.capacity - self.tx.available();
        let buffer_required = self.buffer_len.load(Ordering::Relaxed) * self.buffer_depth;

        (used < buffer_required).then_some(buffer_required.saturating_sub(used))
    }

    pub fn wants_sample_count(&self, samples: usize) -> bool {
        let used = self.capacity - self.tx.available();
        used < samples
    }

    pub fn wait_for_wants_samples(&self, duration: std::time::Duration) -> Option<usize> {
        if let Some(samples) = self.wants_samples() {
            return Some(samples);
        }

        self.notify.wait_timeout(duration);
        self.wants_samples()
    }
}

fn process_samples(wav: Option<&mut WavWriter>, samples: &[i16]) {
    if let Some(wav) = wav {
        if let Err(err) = wav.write_samples(samples) {
            tracing::error!("wav recording: {err:?}");
        }
    }
}

#[derive(Clone)]
struct SamplesNotify {
    pair: Arc<(Mutex<()>, Condvar)>,
}

impl SamplesNotify {
    fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(()), Condvar::new())),
        }
    }

    fn notify(&self) {
        let (_lock, cvar) = &*self.pair;
        cvar.notify_all()
    }

    fn wait_timeout(&self, duration: std::time::Duration) {
        let (lock, cvar) = &*self.pair;
        let guard = lock.lock().unwrap();
        let _ = cvar.wait_timeout(guard, duration).unwrap();
    }
}

macro_rules! impl_audio_devices {
    {$($(#[$attr:meta])? $variant:ident => $struct:ident;)*} => {
        pub enum AudioDevices {
            $(
                $(#[$attr])?
                $variant($struct)
            ),*
        }

        impl Audio for AudioDevices {
            fn sample_rate(&self) -> u32 {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.sample_rate()),*
                }
            }

            fn play(&mut self) {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.play()),*
                }
            }

            fn pause(&mut self) {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.pause()),*
                }
            }

            fn volume(&mut self, volume: f32) {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.volume(volume)),*
                }
            }
        }

        $(
            $(#[$attr])?
            impl From<$struct> for AudioDevices {
                fn from(value: $struct) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

impl_audio_devices! {
    #[cfg(not(target_arch = "wasm32"))] Cpal => CpalAudio;
    #[cfg(all(not(target_arch = "wasm32"), feature = "jack"))] Jack => JackAudio;
    #[cfg(all(not(target_arch = "wasm32"), feature = "pipewire"))] Pipewire => PipewireAudio;
    #[cfg(target_arch = "wasm32")] Browser => BrowserAudio;
    Null => Null;
}

pub fn worket_module_source(wasm_module_path: &str) -> String {
    let mut module = format!("import init, * as wasm from '{wasm_module_path}';");
    module.push_str(WORKLET_REGISTER_JS);
    module
}

const WORKLET_REGISTER_JS: &'static str = r#"

registerProcessor("WorkletProcessor", class WasmProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        let {module, memory, ptr} = options.processorOptions;
        wasm.initSync({ module, memory });
        this.processor = wasm.WorkletProcessor.unpack(ptr);
    }
    process(inputs, outputs) {
        return this.processor.process(outputs[0][0]);
    }
});
"#;
