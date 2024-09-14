mod helper;

const DIR: &'static str = "branch_timing_tests/";

#[test]
fn branch_basics() {
    helper::run(
        format!("{}1.Branch_Basics.nes", DIR),
        helper::RunUntil::Frame(10),
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn backward_branch() {
    helper::run(
        format!("{}2.Backward_Branch.nes", DIR),
        helper::RunUntil::Frame(20),
        helper::Condition::Equals(0xf8, 0x01),
    );
}

#[test]
fn forward_branch() {
    helper::run(
        format!("{}3.Forward_Branch.nes", DIR),
        helper::RunUntil::Frame(20),
        helper::Condition::Equals(0xf8, 0x01),
    );
}
