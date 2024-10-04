use crate::helper::{self, rom};

const DIR: &'static str = "cpu_interrupts_v2/rom_singles/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
fn cli_latency() {
    run("1-cli_latency.nes");
}

#[test]
fn nmi_and_brk() {
    run("2-nmi_and_brk.nes");
}

#[test]
fn nmi_and_irq() {
    run("3-nmi_and_irq.nes");
}

#[test]
fn irq_and_dma() {
    run("4-irq_and_dma.nes");
}

#[test]
fn branch_delays_irq() {
    run("5-branch_delays_irq.nes");
}
