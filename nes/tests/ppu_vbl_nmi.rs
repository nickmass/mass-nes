mod helper;

const DIR: &'static str = "ppu_vbl_nmi/rom_singles/";

#[test]
fn vbl_basics() {
    helper::run(
        format!("{}01-vbl_basics.nes", DIR),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn vbl_set_time() {
    helper::run(
        format!("{}02-vbl_set_time.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn vbl_clear_time() {
    helper::run(
        format!("{}03-vbl_clear_time.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn nmi_control() {
    helper::run(
        format!("{}04-nmi_control.nes", DIR),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn nmi_timing() {
    helper::run(
        format!("{}05-nmi_timing.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn suppression() {
    helper::run(
        format!("{}06-suppression.nes", DIR),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn nmi_on_timing() {
    helper::run(
        format!("{}07-nmi_on_timing.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn nmi_off_timing() {
    helper::run(
        format!("{}08-nmi_off_timing.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn even_odd_frames() {
    helper::run(
        format!("{}09-even_odd_frames.nes", DIR),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
fn even_odd_timing() {
    helper::run(
        format!("{}10-even_odd_timing.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}
