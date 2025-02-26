use crate::helper;

const DIR: &'static str = "blargg_apu_2005.07.30/";

#[test]
fn len_ctr() {
    helper::run(
        format!("{}01.len_ctr.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn len_table() {
    helper::run(
        format!("{}02.len_table.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn irq_flag() {
    helper::run(
        format!("{}03.irq_flag.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn clock_jitter() {
    helper::run(
        format!("{}04.clock_jitter.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn len_timing_mode0() {
    helper::run(
        format!("{}05.len_timing_mode0.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn len_timing_mode1() {
    helper::run(
        format!("{}06.len_timing_mode1.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn irq_flag_timing() {
    helper::run(
        format!("{}07.irq_flag_timing.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn irq_timing() {
    helper::run(
        format!("{}08.irq_timing.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn reset_timing() {
    helper::run(
        format!("{}09.reset_timing.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn len_halt_timing() {
    helper::run(
        format!("{}10.len_halt_timing.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}

#[test]
fn len_reload_timing() {
    helper::run(
        format!("{}11.len_reload_timing.nes", DIR),
        helper::RunUntil::Frame(50),
        helper::Condition::Equals(0x00F0, 0x01),
    );
}
