mod helper;

const DIR: &'static str = "nestest/";

#[test]
#[ignore]
fn nestest_no_ppu() {
    helper::run(
        format!("{}nestest.nes", DIR),
        1,
        helper::Condition::PowerUpPc(0xc000),
    );
}
