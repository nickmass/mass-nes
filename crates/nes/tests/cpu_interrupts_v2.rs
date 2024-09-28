mod helper;

const DIR: &'static str = "cpu_interrupts_v2/rom_singles/";

#[test]
fn cli_latency() {
    helper::run(
        format!("{}1-cli_latency.nes", DIR),
        helper::RunUntil::Frame(100),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
#[should_panic]
fn nmi_and_brk() {
    helper::run(
        format!("{}2-nmi_and_brk.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
#[should_panic]
fn nmi_and_irq() {
    helper::run(
        format!("{}3-nmi_and_irq.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
#[should_panic]
fn irq_and_dma() {
    helper::run(
        format!("{}4-irq_and_dma.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}

#[test]
#[should_panic]
fn branch_delays_irq() {
    helper::run(
        format!("{}5-branch_delays_irq.nes", DIR),
        helper::RunUntil::Frame(300),
        helper::Condition::Equals(0x000a, 0x00),
    );
}
