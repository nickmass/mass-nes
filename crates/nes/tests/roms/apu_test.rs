use crate::helper::{self, rom};

const DIR: &'static str = "apu_test/rom_singles/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn len_ctr() {
    run("1-len_ctr.nes");
}

#[test]
fn len_table() {
    run("2-len_table.nes");
}

#[test]
fn irq_flag() {
    run("3-irq_flag.nes");
}

#[test]
fn jitter() {
    run("4-jitter.nes");
}

#[test]
fn len_timing() {
    run("5-len_timing.nes");
}

#[test]
fn irq_flag_timing() {
    run("6-irq_flag_timing.nes");
}

#[test]
fn dmc_basics() {
    run("7-dmc_basics.nes");
}

#[test]
fn dmc_rates() {
    run("8-dmc_rates.nes");
}
