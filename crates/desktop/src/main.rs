use blip_buf_rs::Blip;
use clap::{Parser, Subcommand, ValueEnum};
use nes::{Cartridge, Machine};
use ui::audio::{Audio, AudioDevices, CpalAudio, Null};
use ui::filters::NesNtscSetup;

use std::{fs::File, path::PathBuf};

pub mod app;
pub mod audio;
pub mod gfx;
pub mod sync;

use app::{App, NesInputs, NesOutputs};
use sync::{NaiveSync, SyncDevices};

fn main() {
    let args = Args::parse();

    use tracing_subscriber::{layer::SubscriberExt, Layer};
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_tracy::TracyLayer::default())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
            ),
    )
    .expect("init tracing");

    match args.mode {
        Mode::Run { file } => run(file, args.region.into()),
        Mode::Bench { frames, file } => bench(file, args.region.into(), frames),
    }
}

fn run(path: PathBuf, region: nes::Region) {
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();

    let setup = NesNtscSetup::composite();
    let filter = ui::filters::NtscFilter::new(&setup);
    //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());

    let sync = audio::CpalSync::new();
    let (audio, frame_sync): (AudioDevices<_>, SyncDevices) =
        match CpalAudio::new(sync, region.refresh_rate(), 64) {
            Ok((audio, frame_sync)) => (audio.into(), frame_sync.into()),
            Err(err) => {
                tracing::error!("unable to init audio device: {err:?}");
                (Null.into(), NaiveSync::new(region.refresh_rate()).into())
            }
        };

    let sample_rate = audio.sample_rate();

    let mut app = App::new(filter, audio, frame_sync);

    let (input, output) = app.nes_io();

    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || run_machine(region, cart, sample_rate, input, output))
        .unwrap();

    app.run();
}

fn run_machine(
    region: nes::Region,
    cart: Cartridge,
    sample_rate: u32,
    input: NesInputs,
    output: NesOutputs,
) {
    let mut machine = instrument_machine(Machine::new(region, cart));
    let mut delta = 0;
    let mut blip = Blip::new(sample_rate / 30);
    blip.set_rates(
        region.frame_ticks() * region.refresh_rate(),
        sample_rate as f64,
    );

    for input in input.inputs() {
        machine.handle_input(input);
        machine.run();

        {
            let span = tracing::trace_span!("audio_blip");
            let _enter = span.enter();

            let samples = machine.get_audio();
            let count = samples.len();

            for (i, v) in samples.iter().enumerate() {
                blip.add_delta(i as u32, *v as i32 - delta);
                delta = *v as i32;
            }
            blip.end_frame(count as u32);
            while blip.samples_avail() > 0 {
                let mut buf = vec![0i16; 1024];
                let count = blip.read_samples(&mut buf, 1024, false);
                buf.truncate(count as usize);
                output.send_samples(buf);
            }
        }

        let frame = machine.get_screen().iter().map(|p| p.get()).collect();
        output.send_frame(frame);
    }
}

fn bench(path: PathBuf, region: nes::Region, mut frames: u32) {
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();
    let mut machine = Machine::new(region, cart);
    loop {
        machine.run();
        frames -= 1;
        if frames == 0 {
            break;
        }
    }
}

#[derive(Parser)]
struct Args {
    /// Selects which console version to emulate
    #[arg(short, long, value_enum, default_value_t)]
    region: Region,
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Debug, Copy, Clone, ValueEnum, Default)]
pub enum Region {
    #[default]
    Ntsc,
    Pal,
}

impl From<Region> for nes::Region {
    fn from(value: Region) -> Self {
        match value {
            Region::Ntsc => nes::Region::Ntsc,
            Region::Pal => nes::Region::Pal,
        }
    }
}

#[derive(Subcommand)]
enum Mode {
    /// Run for specified number of frames with ui
    Bench {
        /// Number of frames to emulate, 0 = infinite
        #[arg(short, long)]
        frames: u32,
        /// Provides a rom file to emulate
        file: PathBuf,
    },
    /// Run a rom
    Run {
        /// Provides a rom file to emulate
        file: PathBuf,
    },
}

fn instrument_machine(machine: Machine) -> Machine {
    if let Some(client) = tracy_client::Client::running() {
        client.plot_config(c"scanline", true, true, None);
        client.plot_config(c"vblank", true, true, None);
        client.plot_config(c"nmi", true, true, None);
    }
    let mut scanline = 0;
    let mut vblank = false;
    let mut nmi = false;
    machine.with_trace_fn(move |_cpu, ppu| {
        if let Some(client) = tracy_client::Client::running() {
            if scanline != ppu.scanline {
                client.plot_int(c"scanline", ppu.scanline as i64);
                scanline = ppu.scanline;
            }
            if vblank != ppu.vblank {
                client.plot_int(c"vblank", ppu.vblank as i64);
                vblank = ppu.vblank;
            }
            if nmi != ppu.triggered_nmi {
                client.plot_int(c"nmi", ppu.triggered_nmi as i64);
                nmi = ppu.triggered_nmi;
            }
        }
    })
}

trait TracyExt {
    fn plot_config(
        &self,
        name: &'static std::ffi::CStr,
        step: bool,
        fill: bool,
        color: Option<u32>,
    );
    fn plot_int(&self, name: &'static std::ffi::CStr, value: i64);
    fn emit_frame_image(&self, data: &[u8], width: u16, height: u16, offset: u8, flip: bool);
}

impl TracyExt for tracy_client::Client {
    fn plot_config(
        &self,
        name: &'static std::ffi::CStr,
        step: bool,
        fill: bool,
        color: Option<u32>,
    ) {
        unsafe {
            tracy_client::sys::___tracy_emit_plot_config(
                name.as_ptr(),
                tracy_client::sys::TracyPlotFormatEnum_TracyPlotFormatNumber as i32,
                step as i32,
                fill as i32,
                color.unwrap_or(0),
            );
        }
    }

    fn plot_int(&self, name: &'static std::ffi::CStr, value: i64) {
        unsafe {
            tracy_client::sys::___tracy_emit_plot_int(name.as_ptr(), value);
        }
    }

    fn emit_frame_image(&self, data: &[u8], width: u16, height: u16, offset: u8, flip: bool) {
        unsafe {
            tracy_client::sys::___tracy_emit_frame_image(
                data.as_ptr() as _,
                width,
                height,
                offset,
                flip as i32,
            );
        }
    }
}
