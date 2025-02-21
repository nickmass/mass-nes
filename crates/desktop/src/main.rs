use clap::{Parser, Subcommand, ValueEnum};
use nes::{Cartridge, Machine};
use runner::Runner;
use ui::audio::{Audio, AudioDevices, CpalAudio, Null, SamplesSender};
use ui::filters::NesNtscSetup;

use std::{fs::File, path::PathBuf};

pub mod app;
pub mod gfx;
mod runner;

use app::App;

fn main() {
    let args = Args::parse();

    init_tracing();

    match args.mode {
        Mode::Run { file } => run(file, args.region.into()),
        Mode::Bench { frames, file } => bench(file, args.region.into(), frames),
    }
}

fn run(path: PathBuf, region: nes::Region) {
    let mut file = File::open(&path).unwrap();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let cart = Cartridge::load(&mut file, None, None, file_name).unwrap();

    let setup = NesNtscSetup::composite();
    let filter = ui::filters::CrtFilter::new(&setup);
    //let filter = ui::filters::NscFilter::new(&setup);
    //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());

    let (audio, samples_tx) = init_audio(region);
    let sample_rate = audio.sample_rate();
    let mut app = App::new(filter, audio);
    let input = app.nes_io();
    let back_buffer = app.back_buffer();

    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || {
            let runner = Runner::new(cart, region, input, back_buffer, samples_tx, sample_rate);

            runner.run()
        })
        .unwrap();

    app.run();
}

fn bench(path: PathBuf, region: nes::Region, mut frames: u32) {
    let mut file = File::open(&path).unwrap();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let cart = Cartridge::load(&mut file, None, None, file_name).unwrap();
    let mut machine = Machine::new(region, cart);
    loop {
        machine.run();
        frames -= 1;
        if frames == 0 {
            break;
        }
    }
}

fn init_audio(region: nes::Region) -> (AudioDevices, SamplesSender) {
    match CpalAudio::new(region.refresh_rate()) {
        _ if std::env::var("MASS_NES_NO_AUDIO").is_ok() => {
            let (audio, tx) = Null::new();
            (audio.into(), tx)
        }
        Ok((audio, samples_tx)) => (audio.into(), samples_tx),
        Err(err) => {
            tracing::error!("unable to init audio device: {err:?}");
            let (audio, tx) = Null::new();
            (audio.into(), tx)
        }
    }
}

fn init_tracing() {
    use tracing::Level;
    use tracing_subscriber::{Layer, filter, layer::SubscriberExt};

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
