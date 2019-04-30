mod helper;

const DIR: &'static str = "sprite_overflow_tests/";

#[test]
fn basics() {
    helper::run(
        format!("{}1.Basics.nes", DIR),
        30,
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn details() {
    helper::run(
        format!("{}2.Details.nes", DIR),
        20,
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn timing() {
    helper::run(
        format!("{}3.Timing.nes", DIR),
        150,
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn obscure() {
    helper::run(
        format!("{}4.Obscure.nes", DIR),
        20,
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn emulator() {
    helper::run(
        format!("{}5.Emulator.nes", DIR),
        20,
        helper::Condition::Equals(0xf8, 0x01),
    );
}
