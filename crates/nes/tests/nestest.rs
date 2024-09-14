mod helper;

const DIR: &'static str = "nestest/";

#[test]
#[ignore]
fn nestest_no_ppu() {
    helper::run(
        format!("{}nestest.nes", DIR),
        helper::RunUntil::Frame(1),
        helper::Condition::PowerUpPc(0xc000),
    );
}
