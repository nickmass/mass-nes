use crate::helper::{self, rom};

const DIR: &'static str = "cpu_dummy_reads/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)),
        helper::RunUntil::Frame(40),
        helper::Condition::Equals(0x0016, 8).with_indirect_message(0x0017),
    );
}

#[test]
fn cpu_dummy_reads() {
    run("cpu_dummy_reads.nes");
}
