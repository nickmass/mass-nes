mod nes;

use nes::system::{System, Region};

fn main() {
    let mut file = ::std::fs::File::open("/home/nickmass/smb.nes").unwrap();

    let cart = System::load_rom(&mut file).unwrap();
    let mut system = System::new(Region::Ntsc, cart);

    system.tick();
}
