use audio::CpalSync;
use clap::{Parser, Subcommand, ValueEnum};
use nes::{Cartridge, Machine};
use runner::Runner;
use ui::audio::{Audio, AudioDevices, CpalAudio, Null, SamplesProducer};
use ui::filters::NesNtscSetup;

use std::{fs::File, path::PathBuf};

pub mod app;
pub mod audio;
pub mod gfx;
mod runner;
pub mod sync;

use app::App;
use sync::{NaiveSync, SyncDevices};

fn main() {
    let args = Args::parse();

    init_tracing();

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

    let (audio, sync, samples_producer) = init_audio(region);
    let sample_rate = audio.sample_rate();
    let mut app = App::new(filter, audio, sync);
    let input = app.nes_io();
    let back_buffer = app.back_buffer();

    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || {
            let runner = Runner::new(
                cart,
                region,
                input,
                back_buffer,
                samples_producer,
                sample_rate,
            );

            runner.run()
        })
        .unwrap();

    app.run();
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

fn init_audio(
    region: nes::Region,
) -> (AudioDevices<CpalSync>, SyncDevices, Option<SamplesProducer>) {
    let sync = audio::CpalSync::new();
    match CpalAudio::new(sync, region.refresh_rate(), 64) {
        Ok((audio, frame_sync, samples_producer)) => {
            (audio.into(), frame_sync.into(), Some(samples_producer))
        }
        Err(err) => {
            tracing::error!("unable to init audio device: {err:?}");
            (
                Null.into(),
                NaiveSync::new(region.refresh_rate()).into(),
                None,
            )
        }
    }
}

fn init_tracing() {
    use tracing::Level;
    use tracing_subscriber::{filter, layer::SubscriberExt, Layer};

    let tracy =
        tracing_tracy::TracyLayer::default().with_filter(filter::Targets::new().with_targets([
            ("mass_nes", Level::TRACE),
            ("nes", Level::TRACE),
            ("ui", Level::TRACE),
        ]));
    let log = tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::DEBUG);

    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(tracy).with(log))
        .expect("init tracing");
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
