use crate::helper::{self, Input, rom};

const DIR: &'static str = "nestest/";

#[test]
#[ignore = "used for manual testing"]
fn nestest_no_ppu() {
    helper::run(
        rom(format!("{}nestest.nes", DIR)).with_power_up_pc(0xc000),
        helper::RunUntil::Frame(1),
        helper::Condition::Equals(0, 0),
    );
}

#[test]
fn nestest_official() {
    helper::run(
        rom(format!("{}nestest.nes", DIR)).with_input([Input::Delay(30), Input::Start]),
        helper::RunUntil::Frame(100),
        helper::Condition::ScreenCrc(0x01D49136),
    );
}

#[test]
fn nestest_unofficial() {
    helper::run(
        rom(format!("{}nestest.nes", DIR)).with_input([
            Input::Delay(30),
            Input::Select,
            Input::Delay(5),
            Input::None,
            Input::Delay(30),
            Input::Start,
        ]),
        helper::RunUntil::Frame(120),
        helper::Condition::ScreenCrc(0xE32B1A5D),
    );
}
