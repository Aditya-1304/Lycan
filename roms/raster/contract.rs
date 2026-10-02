//! Slice-25 HBlank-DMA raster contract.
//!
//! The oracle derives the expected effect from the destination scanline, not
//! from the guest's generated DMA table. This makes event-order failures visible:
//! a transfer that lands one line early or late changes an entire horizontal band.

use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};

const BASE_COLOR: u16 = 0x4210;

fn brighten(color: u16, evy: u8) -> u16 {
    let evy = u32::from(evy.min(16));

    let mut r = u32::from(color & 31);
    let mut g = u32::from(((color >> 4) & 62) | (color >> 15));
    let mut b = u32::from((color >> 10) & 31);

    r += ((31 - r) * evy + 8) >> 4;
    g += ((63 - g) * evy + 8) >> 4;
    b += ((31 - b) * evy + 8) >> 4;

    (r | ((g >> 1) << 5) | (b << 10)) as u16 & 0x7fff
}

fn level(state: usize, y: usize) -> u8 {
    let forward = (y / 10).min(15) as u8;

    match state {
        0 => forward,
        1 | 2 => 15 - forward,
        _ => unreachable!("three raster states"),
    }
}

fn expected_flags(state: usize) -> u16 {
    match state {
        0 => 0x0100,
        1 => 0x0800,
        2 => 0x0900,
        _ => unreachable!("three raster states"),
    }
}

fn expected_alpha_sentinel(state: usize) -> u16 {
    match state {
        0 => 0x0404,
        1 | 2 => 0x0c0c,
        _ => unreachable!("three raster states"),
    }
}

fn initial_brightness(state: usize) -> u16 {
    match state {
        0 => 0,
        1 | 2 => 15,
        _ => unreachable!("three raster states"),
    }
}

fn verify_frame(
    machine: &Machine,
    state: usize,
    frame: u64,
    capture: &mut impl FnMut(u64, &[u16]),
) {
    assert_eq!(
        machine.framebuffer_generation(),
        frame,
        "unexpected framebuffer generation"
    );

    assert!(
        machine.cycles().0 <= frame * CYCLES_PER_FRAME + 32,
        "frame {frame} exceeded instruction-boundary overshoot: {}",
        machine.cycles().0
    );

    assert!(
        (0x0800_0000..0x0800_1000).contains(&machine.registers()[15]),
        "guest left raster ROM at frame {frame}: PC={:#010x}",
        machine.registers()[15]
    );

    assert_eq!(
        machine.inspect16(0x0300_0000).unwrap(),
        0x00a6,
        "completion mailbox ID"
    );

    assert_eq!(
        machine.inspect16(0x0300_0002).unwrap(),
        state as u16,
        "guest raster state"
    );

    assert_eq!(
        machine.inspect16(0x0300_0004).unwrap(),
        expected_flags(state),
        "DMA completion flags do not match active channels"
    );

    /*
     * VBlank re-arms the descriptor and writes line-0 state explicitly.
     * At the frame boundary these values therefore describe the next field's
     * starting effect before line 0's HBlank has occurred.
     */
    assert_eq!(
        machine.inspect16(0x0400_0052).unwrap(),
        expected_alpha_sentinel(state),
        "unexpected winning DMA sentinel"
    );

    assert_eq!(
        machine.inspect16(0x0400_0054).unwrap(),
        initial_brightness(state),
        "unexpected line-0 brightness"
    );

    for y in 0..160usize {
        let expected = brighten(BASE_COLOR, level(state, y));

        for x in 0..240usize {
            assert_eq!(
                machine.framebuffer()[y * 240 + x],
                expected,
                "frame={frame} state={state} x={x} y={y}"
            );
        }
    }

    capture(frame, machine.framebuffer());

    println!(
        "raster frame={frame} state={state} flags={:#06x} \
         pc={:#010x} cycles={} instructions={} PASS",
        expected_flags(state),
        machine.registers()[15],
        machine.cycles().0,
        machine.executed_instructions(),
    );
}

pub fn verify_with_capture(mut capture: impl FnMut(u64, &[u16])) {
    let mut machine = Machine::new();

    machine.load_rom(include_bytes!("../raster.gba")).unwrap();

    let mut frame = 12u64;

    machine
        .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
        .unwrap();

    verify_frame(&machine, 0, frame, &mut capture);

    frame += 1;

    machine
        .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
        .unwrap();

    verify_frame(&machine, 0, frame, &mut capture);

    for state in 1usize..=2 {
        machine.set_button(Button::Right, true);

        frame += 2;

        machine
            .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
            .unwrap();

        machine.set_button(Button::Right, false);

        frame += 2;

        machine
            .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
            .unwrap();

        verify_frame(&machine, state, frame, &mut capture);

        frame += 1;

        machine
            .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
            .unwrap();

        verify_frame(&machine, state, frame, &mut capture);
    }
}
