use crate::helper::{self, rom};

const DIR: &'static str = "pal_apu_tests/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_region(nes::Region::Pal),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn len_ctr() {
    run("01.len_ctr.nes");
}

#[test]
fn len_table() {
    run("02.len_table.nes");
}

#[test]
fn irq_flag() {
    run("03.irq_flag.nes");
}

#[test]
#[should_panic]
fn clock_jitter() {
    run("04.clock_jitter.nes");
}

#[test]
#[should_panic]
fn len_timing_mode0() {
    run("05.len_timing_mode0.nes");
}

#[test]
#[should_panic]
fn len_timing_mode1() {
    run("06.len_timing_mode1.nes");
}

#[test]
#[should_panic]
fn irq_flag_timing() {
    run("07.irq_flag_timing.nes");
}

#[test]
#[should_panic]
fn irq_timing() {
    run("08.irq_timing.nes");
}

#[test]
#[should_panic]
fn len_halt_timing() {
    run("10.len_halt_timing.nes");
}

#[test]
#[should_panic]
fn len_reload_timing() {
    run("11.len_reload_timing.nes");
}
