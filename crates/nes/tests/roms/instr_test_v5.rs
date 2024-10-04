use crate::helper::{self, rom};

const DIR: &'static str = "instr_test-v5/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn official_only() {
    run("official_only.nes");
}

#[test]
fn all_instrs() {
    run("all_instrs.nes");
}
