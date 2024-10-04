use crate::helper::{self, rom};

const DIR: &'static str = "instr_misc/rom_singles/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn abs_x_wrap() {
    run("01-abs_x_wrap.nes");
}

#[test]
fn branch_wrap() {
    run("02-branch_wrap.nes");
}

#[test]
fn dummy_reads() {
    run("03-dummy_reads.nes");
}

#[test]
fn dummy_reads_apu() {
    run("04-dummy_reads_apu.nes");
}
