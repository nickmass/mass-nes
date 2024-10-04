use crate::helper::{self, rom};

const DIR: &'static str = "oam_read/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn oam_read() {
    run("oam_read.nes");
}
