use direct_ring_buffer::{Consumer, Producer, create_ring_buffer};

#[cfg(not(target_arch = "wasm32"))]
mod cpal;
#[cfg(not(target_arch = "wasm32"))]
pub use cpal::CpalAudio;

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserAudio;

mod null;
pub use null::Null;

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn play(&mut self);
    fn pause(&mut self);
    fn volume(&mut self, volume: f32);
}

fn samples_channel(capacity: usize, target_buffer: usize) -> (SamplesSender, SamplesReceiver<i16>) {
    let (tx, rx) = create_ring_buffer(capacity);

    let rx = SamplesReceiver { rx, last_sample: 0 };
    let tx = SamplesSender {
        tx,
        capacity,
        target_samples: target_buffer,
    };

    (tx, rx)
}

struct SamplesReceiver<T> {
    rx: Consumer<T>,
    last_sample: T,
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
    target_samples: usize,
}

impl SamplesSender {
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

    pub fn add_samples_from_blip(&mut self, blip: &mut blip_buf_rs::Blip) {
        let avail = blip.samples_avail() as usize;
        let written = self.tx.write_slices(
            |buf, _offset| blip.read_samples(buf, buf.len() as u32, false) as usize,
            Some(avail),
        );

        if written < avail {
            tracing::warn!("run out of sample space");
            blip.clear();
        }
    }

    pub fn wants_samples(&self) -> bool {
        let used = self.capacity - self.tx.available();
        used <= self.target_samples
    }
}

pub enum AudioDevices {
    #[cfg(not(target_arch = "wasm32"))]
    Cpal(CpalAudio),
    #[cfg(target_arch = "wasm32")]
    Browser(BrowserAudio),
    Null(Null),
}

impl Audio for AudioDevices {
    fn sample_rate(&self) -> u32 {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioDevices::Cpal(a) => a.sample_rate(),
            #[cfg(target_arch = "wasm32")]
            AudioDevices::Browser(a) => a.sample_rate(),
            AudioDevices::Null(a) => a.sample_rate(),
        }
    }

    fn play(&mut self) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioDevices::Cpal(a) => a.play(),
            #[cfg(target_arch = "wasm32")]
            AudioDevices::Browser(a) => a.play(),
            AudioDevices::Null(a) => a.play(),
        }
    }

    fn pause(&mut self) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioDevices::Cpal(a) => a.pause(),
            #[cfg(target_arch = "wasm32")]
            AudioDevices::Browser(a) => a.pause(),
            AudioDevices::Null(a) => a.pause(),
        }
    }

    fn volume(&mut self, volume: f32) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            AudioDevices::Cpal(a) => a.volume(volume),
            #[cfg(target_arch = "wasm32")]
            AudioDevices::Browser(a) => a.volume(volume),
            AudioDevices::Null(a) => a.volume(volume),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<CpalAudio> for AudioDevices {
    fn from(value: CpalAudio) -> Self {
        AudioDevices::Cpal(value)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<BrowserAudio> for AudioDevices {
    fn from(value: BrowserAudio) -> Self {
        AudioDevices::Browser(value)
    }
}

impl From<Null> for AudioDevices {
    fn from(value: Null) -> Self {
        AudioDevices::Null(value)
    }
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
