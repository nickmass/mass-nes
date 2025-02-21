use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::u32;

use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{Array, Object, Reflect};
use web_sys::wasm_bindgen::prelude::*;
use web_sys::{AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, wasm_bindgen};

use super::{SamplesReceiver, SamplesSender, samples_channel};

pub struct BrowserAudio {
    ctx: AudioContext,
    sample_rate: f32,
    volume: Volume,
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub enum Error {
    CreateContext(JsValue),
    AddWorklet(JsValue),
    ConnectNode(JsValue),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CreateContext(e) => write!(f, "create AudioContext failed: {:?}", e),
            Error::AddWorklet(e) => write!(f, "add Worklet module failed: {:?}", e),
            Error::ConnectNode(e) => write!(f, "connect AudioWorkletNode failed: {:?}", e),
        }
    }
}

impl BrowserAudio {
    pub async fn new(
        worklet_path: &str,
        refresh_rate: f64,
    ) -> Result<(Self, SamplesSender), Error> {
        let ctx = AudioContext::new().map_err(Error::CreateContext)?;
        let volume = Volume::new();
        let sample_rate = ctx.sample_rate();
        let buffer = (sample_rate as f64 / refresh_rate) * 2.1;
        let (samples_tx, samples_rx) = samples_channel(sample_rate as usize, buffer as usize);

        let worklet_processor = WorkletProcessor::new(samples_rx, volume.clone());
        Self::install(&ctx, &worklet_path, worklet_processor).await?;

        Ok((
            Self {
                ctx,
                sample_rate,
                volume,
            },
            samples_tx,
        ))
    }

    async fn install(
        ctx: &AudioContext,
        worklet_path: &str,
        worklet_processor: WorkletProcessor,
    ) -> Result<(), Error> {
        let worklet = ctx.audio_worklet().map_err(Error::AddWorklet)?;
        JsFuture::from(
            worklet
                .add_module(worklet_path)
                .map_err(Error::AddWorklet)?,
        )
        .await
        .map_err(Error::AddWorklet)?;

        let worklet_options = AudioWorkletNodeOptions::new();
        worklet_options.set_number_of_inputs(0);
        worklet_options.set_number_of_outputs(1);
        worklet_options.set_output_channel_count(&Array::of1(&(1.into())));

        let processor_options = Object::new();
        Reflect::set(
            &processor_options,
            &JsValue::from_str("memory"),
            &wasm_bindgen::memory(),
        )
        .map_err(Error::AddWorklet)?;
        Reflect::set(
            &processor_options,
            &JsValue::from_str("module"),
            &wasm_bindgen::module(),
        )
        .map_err(Error::AddWorklet)?;

        let ptr = worklet_processor.pack();
        Reflect::set(&processor_options, &JsValue::from_str("ptr"), &(ptr).into())
            .map_err(Error::AddWorklet)?;
        worklet_options.set_processor_options(Some(&processor_options));

        let worklet_node =
            AudioWorkletNode::new_with_options(&ctx, "WorkletProcessor", &worklet_options)
                .map_err(Error::ConnectNode)?;

        worklet_node
            .connect_with_audio_node(&ctx.destination())
            .map_err(Error::ConnectNode)?;

        Ok(())
    }
}

impl super::Audio for BrowserAudio {
    fn sample_rate(&self) -> u32 {
        self.sample_rate as u32
    }

    fn play(&mut self) {
        let ctx = self.ctx.clone();
        let play = async move {
            if let Ok(fut) = ctx.resume() {
                let _ = JsFuture::from(fut).await;
            }
        };
        wasm_bindgen_futures::spawn_local(play);
    }

    fn pause(&mut self) {
        let ctx = self.ctx.clone();
        let pause = async move {
            if let Ok(fut) = ctx.suspend() {
                let _ = JsFuture::from(fut).await;
            }
        };
        wasm_bindgen_futures::spawn_local(pause);
    }

    fn volume(&mut self, volume: f32) {
        self.volume.set(volume);
    }
}

#[derive(Debug, Clone)]
struct Volume {
    inner: Arc<AtomicU32>,
}

impl Volume {
    fn new() -> Self {
        let inner = Arc::new(AtomicU32::new(u32::MAX));

        Self { inner }
    }

    fn set(&self, volume: f32) {
        let volume = volume.min(1.0).max(0.0);
        let volume = ((u32::MAX as f32) * volume) as u32;
        self.inner.store(volume, Ordering::Relaxed);
    }

    fn get(&self) -> f32 {
        let volume = self.inner.load(Ordering::Relaxed) as f32;
        (volume / (u32::MAX as f32)) as f32
    }
}

#[wasm_bindgen]
pub struct WorkletProcessor {
    samples: SamplesReceiver<i16>,
    volume: Volume,
}

#[wasm_bindgen]
impl WorkletProcessor {
    fn new(samples: SamplesReceiver<i16>, volume: Volume) -> Self {
        Self { samples, volume }
    }

    #[wasm_bindgen]
    pub fn process(&mut self, out: &mut [f32]) -> bool {
        let volume = self.volume.get();
        for (out, sample) in out.iter_mut().zip(&mut self.samples) {
            let sample = (sample as f32) / (i16::MAX as f32);
            *out = sample * volume;
        }

        true
    }

    fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }

    #[wasm_bindgen]
    pub unsafe fn unpack(ptr: usize) -> WorkletProcessor {
        *Box::from_raw(ptr as *mut _)
    }
}
