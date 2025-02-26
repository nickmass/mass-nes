use crate::helper::{self, rom};

const DIR: &'static str = "mmc3_test_2/rom_singles/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn clocking() {
    run("1-clocking.nes");
}

#[test]
fn details() {
    run("2-details.nes");
}

#[test]
fn a12_clocking() {
    run("3-A12_clocking.nes");
}

#[test]
fn scanline_timing() {
    run("4-scanline_timing.nes");
}

#[test]
fn mmc3() {
    run("5-MMC3.nes");
}

#[test]
fn mmc3_alt() {
    run("6-MMC3_alt (Submapper 4).nes");
}
