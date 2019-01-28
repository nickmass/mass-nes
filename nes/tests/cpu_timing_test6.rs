
mod helper;

const DIR: &'static str = "cpu_timing_test6/";

#[test]
fn cpu_timing_test() {
    helper::run(format!("{}cpu_timing_test.nes", DIR), 660,
    helper::Condition::Equals(0x14, 0x02));
}
