use crate::helper::{self, Input, rom};

macro_rules! test_fn(
    ($(#[$attr:meta])? $fn_name:ident, $page:literal, $test:literal, $result:literal $(, $values:literal)*) => {
        #[test]
        $(#[$attr])?
        fn $fn_name() {
            let values = &[0x01,
                $(
                    $values,
                )*
            ];

            run_test($page, $test, $result, values);
        }
    }
);

macro_rules! test_page_fn(
    ($(#[$attr:meta])? $fn_name:ident, $page:literal, $results:expr $(, $values:literal)*) => {
        #[test]
        $(#[$attr])?
        fn $fn_name() {
            let values = &[0x01,
                $(
                    $values,
                )*
            ];

            run_test_page($page, $results, values);
        }
    }
);

// CPU Behavior
test_fn!(rom_is_not_writable, 1, 1, 0x405);
test_fn!(ram_mirroring, 1, 2, 0x403);
test_fn!(ffff_x_wraparound, 1, 3, 0x44d);
test_fn!(ppu_register_mirroring, 1, 4, 0x404);
test_fn!(ppu_register_open_bus, 1, 5, 0x44e);
test_fn!(dummy_read_cycles, 1, 6, 0x406);
test_fn!(dummy_write_cycles, 1, 7, 0x407);
test_fn!(open_bus, 1, 8, 0x408);
test_fn!(unofficial_instr, 1, 9, 0x402);

// Unofficial Instructions: SLO
test_page_fn!(unofficial_slo, 2, 0x409..=0x40f);

// Unofficial Instructions: RLA
test_page_fn!(unofficial_rla, 3, 0x419..=0x41f);

// Unofficial Instructions: SRE
test_page_fn!(unofficial_sre, 4, 0x420..=0x426);

// Unofficial Instructions: RRA
test_page_fn!(unofficial_rra, 5, 0x427..=0x42d);

// Unofficial Instructions: *AX
test_page_fn!(unofficial_sax_lax, 6, 0x42e..=0x437);

// Unofficial Instructions: DCP
test_page_fn!(unofficial_dcp, 7, 0x438..=0x43e);

// Unofficial Instructions: ISC
test_page_fn!(unofficial_isc, 8, 0x43f..=0x445);

// Unofficial Instructions: SH*
test_page_fn!(unofficial_sh_family, 9, 0x446..=0x44b, 0x05);

// Unofficial Immediates
test_page_fn!(unofficial_immediates, 10, 0x410..=0x417);

// CPU Interrupts
test_fn!(i_flag_letency, 11, 1, 0x461);
test_fn!(nmi_and_break, 11, 2, 0x462);
test_fn!(nmi_and_irq, 11, 3, 0x463);

// APU Registers and DMA Tests
test_fn!(dma_open_bus, 12, 1, 0x46c);
test_fn!(dma_2007_read, 12, 2, 0x44c);
test_fn!(dma_2007_write, 12, 3, 0x44f);
test_fn!(dma_4015_read, 12, 4, 0x45d);
test_fn!(dma_4016_read, 12, 5, 0x45e);
test_fn!(controller_strobing, 12, 6, 0x45f);
test_fn!(
    #[should_panic]
    apu_register_activation,
    12,
    7,
    0x45c
);
test_fn!(
    #[should_panic]
    dmc_dma_bus_conflicts,
    12,
    8,
    0x46b
);

// APU Timing
test_fn!(length_counter, 13, 1, 0x465);
test_fn!(length_table, 13, 2, 0x466);
test_fn!(frame_counter_irq, 13, 3, 0x467);
test_fn!(frame_counter_4step, 13, 4, 0x468);
test_fn!(frame_counter_5step, 13, 5, 0x469);
test_fn!(delta_modulation_channel, 13, 6, 0x46a);

// PPU Vblank Timing
test_fn!(vblank_beginning, 15, 1, 0x450);
test_fn!(vblank_end, 15, 2, 0x451);
test_fn!(nmi_control, 15, 3, 0x452);
test_fn!(nmi_timing, 15, 4, 0x453);
test_fn!(nmi_suppression, 15, 5, 0x454);
test_fn!(nmi_at_vblank_end, 15, 6, 0x455);
test_fn!(nmi_disabled_at_vblank, 15, 7, 0x456);
test_fn!(instruction_timing, 15, 8, 0x460);

// Sprite Evaluation
test_fn!(sprite_0_hit, 16, 1, 0x457);
test_fn!(arbitrary_sprite_0, 16, 2, 0x458);
test_fn!(sprite_overflow, 16, 3, 0x459);
test_fn!(misalign_oam, 16, 4, 0x45a);
test_fn!(address_2004, 16, 5, 0x45b);

// PPU Misc
test_fn!(rmw_2007_extra_write, 17, 1, 0x464);

// CPU Behavior 2
test_fn!(implied_dummy_read, 18, 1, 0x46d);

fn run_test(page: u32, test: u32, result: u16, values: &[u8]) {
    let page = page - 1;
    let mut input = Vec::new();
    input.push(Input::Delay(30));

    for _ in 0..page {
        input.push(Input::Right);
        input.push(Input::Delay(1));
        input.push(Input::None);
        input.push(Input::Delay(5));
    }

    for _ in 0..test {
        input.push(Input::Down);
        input.push(Input::Delay(1));
        input.push(Input::None);
        input.push(Input::Delay(5));
    }

    input.push(Input::A);

    let condition = if values.len() == 1 {
        helper::Condition::Equals(result, values[0])
    } else {
        helper::Condition::Any(result, values.to_vec())
    };

    helper::run(
        rom("AccuracyCoin/AccuracyCoin.nes").with_input(input),
        helper::RunUntil::NotEqual(result, 0x03).with_frame_limit(1000),
        condition,
    );
}

fn run_test_page<I: IntoIterator<Item = u16>>(page: u32, results: I, values: &[u8]) {
    for (idx, result) in results.into_iter().enumerate() {
        let test = idx + 1;
        run_test(page, test as u32, result, values);
    }
}
