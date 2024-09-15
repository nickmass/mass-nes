use blip_buf::BlipBuf;
use clap::{Parser, Subcommand, ValueEnum};
use nes::{Cartridge, Machine};
use ui::audio::{Audio, AudioDevices, CpalAudio, Null};
use ui::filters::NesNtscSetup;

use std::{fs::File, path::PathBuf};

pub mod app;
pub mod audio;
pub mod gfx;
pub mod sync;

use app::App;
use sync::{NaiveSync, SyncDevices};

fn main() {
    let args = Args::parse();
    match args.mode {
        Mode::Run { file } => run(file, args.region.into()),
        Mode::Bench { frames, file } => bench(file, args.region.into(), frames),
    }
}

fn run(path: PathBuf, region: nes::Region) {
    let mut file = File::open(path).unwrap();
    let cart = Cartridge::load(&mut file).unwrap();

    let setup = NesNtscSetup::composite();
    let filter = ui::filters::NtscFilter::new(setup);
    //let filter = ui::filters::PalettedFilter::new(setup.generate_palette());

    let sync = audio::CpalSync::new();
    let (audio, frame_sync): (AudioDevices<_>, SyncDevices) =
        match CpalAudio::new(sync, region.refresh_rate(), 64) {
            Ok((audio, frame_sync)) => (audio.into(), frame_sync.into()),
            Err(err) => {
                eprintln!("unable to init audio device: {err:?}");
                (Null.into(), NaiveSync::new(region.refresh_rate()).into())
            }
        };

    let sample_rate = audio.sample_rate();

    let mut app = App::new(filter, audio, frame_sync);

    let (input, output) = app.nes_io();

    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || {
            let mut machine = Machine::new(region, cart);
            let mut delta = 0;
            let mut blip = BlipBuf::new(sample_rate / 30);
            blip.set_rates(
                region.frame_ticks() * region.refresh_rate(),
                sample_rate as f64,
            );

            for input in input.inputs() {
                machine.handle_input(input);
                machine.run();

                let samples = machine.get_audio();
                let count = samples.len();

                for (i, v) in samples.iter().enumerate() {
                    blip.add_delta(i as u32, *v as i32 - delta);
                    delta = *v as i32;
                }
                blip.end_frame(count as u32);
                while blip.samples_avail() > 0 {
                    let mut buf = vec![0i16; 1024];
                    let count = blip.read_samples(&mut buf, false);
                    buf.truncate(count);
                    output.send_samples(buf);
                }

                let frame = machine.get_screen().iter().map(|p| p.get()).collect();
                output.send_frame(frame);
            }
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
