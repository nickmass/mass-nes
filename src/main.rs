mod nes;

use nes::system::{Machine, System, Region};

fn main() {
    let mut file = ::std::fs::File::open("/home/nickmass/smb.nes").unwrap();

    let cart = Machine::load_rom(&mut file).unwrap();
    let mut system = Machine::new(Region::Ntsc, cart);

    system.tick();
}
