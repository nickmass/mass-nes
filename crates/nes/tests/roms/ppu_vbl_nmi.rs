use crate::helper::{self, rom};

const DIR: &'static str = "ppu_vbl_nmi/rom_singles/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn vbl_basics() {
    run("01-vbl_basics.nes");
}

#[test]
fn vbl_set_time() {
    run("02-vbl_set_time.nes");
}

#[test]
fn vbl_clear_time() {
    run("03-vbl_clear_time.nes");
}

#[test]
fn nmi_control() {
    run("04-nmi_control.nes");
}

#[test]
fn nmi_timing() {
    run("05-nmi_timing.nes");
}

#[test]
fn suppression() {
    run("06-suppression.nes");
}

#[test]
fn nmi_on_timing() {
    run("07-nmi_on_timing.nes");
}

#[test]
fn nmi_off_timing() {
    run("08-nmi_off_timing.nes");
}

#[test]
fn even_odd_frames() {
    run("09-even_odd_frames.nes");
}

#[test]
#[should_panic]
fn even_odd_timing() {
    run("10-even_odd_timing.nes");
}
