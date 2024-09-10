use blip_buf::BlipBuf;

use nes::{Cartridge, Machine};

mod ui;
use ui::{
    audio::{Audio, AudioDevices, CpalAudio, Null},
    sync::{NaiveSync, SyncDevices},
};

use nes_ntsc::NesNtscSetup;

use clap::{Parser, Subcommand, ValueEnum};

use std::{fs::File, path::PathBuf};

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

    let filter = ui::ntsc::NtscFilter::new(NesNtscSetup::composite());
    //let filter = ui::gfx::PalettedFilter::new(NesNtscSetup::rgb().generate_palette());

    let (audio, frame_sync): (AudioDevices, SyncDevices) =
        match CpalAudio::new(region.refresh_rate()) {
            Ok((audio, frame_sync)) => (audio.into(), frame_sync.into()),
            Err(err) => {
                eprintln!("unable to init audio device: {err:?}");
                (Null.into(), NaiveSync::new(region.refresh_rate()).into())
            }
        };

    let sample_rate = audio.sample_rate();

    let mut app = ui::window::App::new(filter, audio, frame_sync);

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

                let frame = machine.get_screen();
                output.send_frame(frame.into());
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

fn generate_pal() {
    let mut new_c = [0; 0x40 * 8 * 3];
    let c = nes::Region::Ntsc.default_palette();
    let emp = 0.1;
    let demp = 0.25;
    for i in 0..8 {
        let mut blue = 1.0;
        let mut green = 1.0;
        let mut red = 1.0;

        if i & 0x01 != 0 {
            red += emp;
            green -= demp;
            blue -= demp;
        }

        if i & 0x02 != 0 {
            green += emp;
            red -= demp;
            blue -= demp;
        }

        if i & 0x04 != 0 {
            blue += emp;
            red -= demp;
            green -= demp;
        }

        let red = if red < 0.0 { 0.0 } else { red };
        let green = if green < 0.0 { 0.0 } else { green };
        let blue = if blue < 0.0 { 0.0 } else { blue };
        for j in 0..0x40 {
            let x = j * 3;
            let final_red = (c[x] as f64 * red).round();
            let final_red = if final_red > 255.0 {
                0xff
            } else {
                final_red as u8
            };
            let final_green = (c[x + 1] as f64 * green).round();
            let final_green = if final_green > 255.0 {
                0xff
            } else {
                final_green as u8
            };
            let final_blue = (c[x + 2] as f64 * blue).round();
            let final_blue = if final_blue > 255.0 {
                0xff
            } else {
                final_blue as u8
            };

            let index = (i * 192) + x;
            new_c[index as usize] = final_red;
            new_c[index as usize + 1] = final_green;
            new_c[index as usize + 2] = final_blue;
        }
    }
    //use std::io::Write;
    //let mut f = std::fs::File::create("emp_pal.pal").unwrap();
    //f.write_all(&new_c);
}
