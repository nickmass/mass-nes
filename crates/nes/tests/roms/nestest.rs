use crate::helper::{self, rom};

const DIR: &'static str = "nestest/";

#[test]
#[ignore]
fn nestest_no_ppu() {
    helper::run(
        rom(format!("{}nestest.nes", DIR)).with_power_up_pc(0xc000),
        helper::RunUntil::Frame(1),
        helper::Condition::Equals(0, 0),
    );
}
