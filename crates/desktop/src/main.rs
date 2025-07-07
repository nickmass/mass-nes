use clap::{Parser, Subcommand, ValueEnum};
use nes::{Cartridge, Machine};
use runner::Runner;
use ui::audio::{Audio, AudioDevices, Null, PipewireAudio, SamplesSender};
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
        Mode::Mdf {
            out_file,
            sample_rate,
        } => mdf(out_file, sample_rate),
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

    let mut setup = NesNtscSetup::composite();
    setup.merge_fields = false;
    let filter = ui::filters::CrtFilter::new(&setup);
    //let filter = ui::filters::NtscFilter::new(&setup);
    //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());

    let (audio, samples_tx) = init_audio();
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

fn mdf(out_file: PathBuf, sample_rate: Option<u32>) {
    let region = nes::Region::Ntsc;

    let system_rate = region.frame_ticks() * region.refresh_rate();
    let wav_rate = sample_rate.unwrap_or(system_rate.ceil() as u32);
    let out_wav = File::create(&out_file).unwrap();
    let mut wav_writer = ui::wav_writer::WavWriter::new(out_wav, wav_rate).unwrap();

    let mut rom = &include_bytes!("../assets/mdfourier4k.nes")[..];
    let cart = Cartridge::load(&mut rom, None, None, "mdfourier.nes").unwrap();
    let mut machine = Machine::new(region, cart);

    let mut blip = if let Some(sample_rate) = sample_rate {
        let mut blip = blip_buf::BlipBuf::new(sample_rate);
        blip.set_rates(system_rate, sample_rate as f64);
        Some(blip)
    } else {
        None
    };
    let mut blip_delta = 0;

    let mut recording = false;
    let mut sample_buf = vec![0; region.frame_ticks() as usize * 4];

    let start_frame = 65;
    let end_frame = start_frame + (111 * 60);

    for frame in 0..end_frame {
        if frame == start_frame {
            recording = true;
            let controller = nes::Controller {
                a: frame == start_frame,
                ..Default::default()
            };

            let input = nes::UserInput::PlayerOne(controller);
            machine.handle_input(input);
        }

        if frame != 0 && frame % (end_frame / 6) == 0 {
            tracing::info!("Frame: {frame}/{end_frame}");
        }

        machine.run();

        let samples = if let Some(blip) = blip.as_mut() {
            let samples = machine.take_samples();
            let count = samples.len();
            for (i, v) in samples.enumerate() {
                blip.add_delta(i as u32, v as i32 - blip_delta);
                blip_delta = v as i32;
            }
            blip.end_frame(count as u32);
            let n = blip.read_samples(&mut sample_buf, false);
            &sample_buf[0..n]
        } else {
            sample_buf.clear();
            sample_buf.extend(machine.take_samples());
            &sample_buf
        };

        if recording {
            wav_writer.write_samples(samples).unwrap();
        }
    }

    wav_writer.finalize().unwrap();
    tracing::info!("Finished: {}", out_file.display());
}

fn init_audio() -> (AudioDevices, SamplesSender) {
    match PipewireAudio::new() {
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
    /// Create a MDFourier recording
    Mdf {
        /// Location to write .wav result
        out_file: PathBuf,
        /// Target sample rate, defaults to raw NES output rate of 1.78mhz with no resampling
        sample_rate: Option<u32>,
    },
}
