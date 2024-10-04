use crate::helper::{self, rom};

const DIR: &'static str = "sprdma_and_dmc_dma/";

fn run(rom_path: &'static str) {
    helper::run(
        rom(format!("{}{}", DIR, rom_path)).with_debug_mem(0x6000, 8),
        helper::RunUntil::NotEqual(0x6000, 0x80).with_frame_limit(200),
        helper::Condition::Equals(0x6000, 0x00).with_message(0x6004),
    );
}

#[test]
#[should_panic]
fn sprdma_and_dmc_dma() {
    run("sprdma_and_dmc_dma.nes");
}

#[test]
#[should_panic]
fn sprdma_and_dmc_dma_512() {
    run("sprdma_and_dmc_dma_512.nes");
}
