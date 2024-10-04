use crate::helper::{self, rom};

const DIR: &'static str = "ppu_open_bus/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
#[should_panic]
fn ppu_open_bus() {
    run("ppu_open_bus.nes");
}
