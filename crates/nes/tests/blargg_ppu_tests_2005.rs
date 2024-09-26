mod helper;

const DIR: &'static str = "blargg_ppu_tests_2005.09.15b/";

#[test]
fn palette_ram() {
    helper::run(
        format!("{}palette_ram.nes", DIR),
        helper::RunUntil::Frame(10),
        helper::Condition::Equals(0xf0, 0x01),
    );
}

#[test]
fn sprite_ram() {
    helper::run(
        format!("{}sprite_ram.nes", DIR),
        helper::RunUntil::Frame(10),
        helper::Condition::Equals(0xf0, 0x01),
    );
}

#[test]
fn vbl_clear_time() {
    helper::run(
        format!("{}vbl_clear_time.nes", DIR),
        helper::RunUntil::Frame(20),
        helper::Condition::Equals(0xf0, 0x01),
    );
}

#[test]
fn vram_access() {
    helper::run(
        format!("{}vram_access.nes", DIR),
        helper::RunUntil::Frame(10),
        helper::Condition::Equals(0xf0, 0x01),
    );
}
