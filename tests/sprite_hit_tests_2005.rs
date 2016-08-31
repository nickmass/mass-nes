
mod helper;

const DIR: &'static str = "sprite_hit_tests_2005.10.05/";

#[test]
fn basics() {
    helper::run(format!("{}01.basics.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn alignment() {
    helper::run(format!("{}02.alignment.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn corners() {
    helper::run(format!("{}03.corners.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn flip() {
    helper::run(format!("{}04.flip.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn left_clip() {
    helper::run(format!("{}05.left_clip.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn right_edge() {
    helper::run(format!("{}06.right_edge.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn screen_bottom() {
    helper::run(format!("{}07.screen_bottom.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn double_height() {
    helper::run(format!("{}08.double_height.nes", DIR), 50,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn timing_basics() {
    helper::run(format!("{}09.timing_basics.nes", DIR), 150,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn timing_order() {
    helper::run(format!("{}10.timing_order.nes", DIR), 150,
    helper::Condition::Equals(0xf8, 0x01));
}

#[test]
fn edge_timing() {
    helper::run(format!("{}11.edge_timing.nes", DIR), 150,
    helper::Condition::Equals(0xf8, 0x01));
}
