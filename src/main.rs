mod nes;

use nes::system::{System, Region};

fn main() {
    let mut file = ::std::fs::File::open("/home/nickmass/smb.nes").unwrap();

    System::load_rom(&mut file);
    let mut system = System::new(Region::Ntsc);

    system.tick();
}
