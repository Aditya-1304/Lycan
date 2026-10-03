//! Bounded guest replay with an independent source-coordinate image oracle.
use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};

/// Checks affine bounds, signed matrices, transparent holes, addressing and BG priority.
/// The source oracle deliberately has no tile, palette or OAM-address calculation.
pub fn verify_with_capture(mut capture: impl FnMut(u64, &[u16])) {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!("../affine-object.gba"))
        .unwrap();
    let mut state = [0u16; 7];
    let mut frame = 20;
    machine
        .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
        .unwrap();
    for button in [
        None,
        Some(Button::Right),
        Some(Button::Up),
        Some(Button::A),
        Some(Button::B),
        Some(Button::R),
        Some(Button::L),
        Some(Button::L),
        Some(Button::Left),
        Some(Button::Right),
        Some(Button::Right),
        Some(Button::R),
        Some(Button::B),
        Some(Button::A),
    ] {
        if let Some(button) = button {
            machine.set_button(button, true);
            frame += 2;
            machine
                .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
                .unwrap();
            machine.set_button(button, false);
            frame += 2;
            machine
                .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
                .unwrap();
            match button {
                Button::Right => state[0] = (state[0] + 1) % 4,
                Button::Up => state[1] ^= 1,
                Button::A => state[2] ^= 1,
                Button::B => state[3] ^= 1,
                Button::R => state[4] ^= 1,
                Button::L => state[5] ^= 1,
                Button::Left => state[6] ^= 1,
                _ => unreachable!(),
            }
        }
        let pc = machine.registers()[15];
        assert!(
            (0x0800017c..=0x08000190).contains(&pc),
            "unexpected PC {pc:#x}"
        );
        assert_eq!(machine.framebuffer_generation(), frame);
        assert!(machine.cycles().0 <= frame * CYCLES_PER_FRAME + 32);
        assert_eq!(machine.inspect16(0x03000000).unwrap(), 0xa2);
        for (slot, expected) in state.iter().enumerate() {
            assert_eq!(
                machine.inspect16(0x03000002 + slot as u32 * 2).unwrap(),
                *expected
            );
        }
        let [angle, enlarged, expanded, _, _, behind, wrapped] = state;
        let mut matrix = match angle {
            0 => [256, 0, 0, 256],
            1 => [181, -181, 181, 181],
            2 => [0, -256, 256, 0],
            3 => [-256, 0, 0, -256],
            _ => unreachable!(),
        };
        if enlarged != 0 {
            matrix = matrix.map(|coefficient| coefficient >> 1);
        }
        let bound = if expanded != 0 { 64 } else { 32 };
        let left = if wrapped != 0 { -8 } else { 100 } - if expanded != 0 { 16 } else { 0 };
        let top = if wrapped != 0 { -8 } else { 60 } - if expanded != 0 { 16 } else { 0 };
        for y in 0..160i32 {
            for x in 0..240i32 {
                let dx = x - left - bound / 2;
                let dy = y - top - bound / 2;
                let sx = ((matrix[0] * dx + matrix[1] * dy) >> 8) + 16;
                let sy = ((matrix[2] * dx + matrix[3] * dy) >> 8) + 16;
                let visible = (left..left + bound).contains(&x)
                    && (top..top + bound).contains(&y)
                    && (0..32).contains(&sx)
                    && (0..32).contains(&sy)
                    && sx >= 6
                    && !(sx < 16 && sy < 12)
                    && behind == 0;
                let expected = if visible {
                    let color = (1 + (sx / 4 + 3 * (sy / 4)) % 15) as u16;
                    color | color << 10
                } else {
                    0x3e0
                };
                assert_eq!(
                    machine.framebuffer()[y as usize * 240 + x as usize],
                    expected,
                    "frame={frame} state={state:?} pixel=({x},{y}) source=({sx},{sy})"
                );
            }
        }
        capture(frame, machine.framebuffer());
        println!(
            "PASS affine-object frame={frame} state={state:?} pc={pc:#x} cycles={} instructions={}",
            machine.cycles().0,
            machine.executed_instructions()
        );
    }
}

/// Runs a standalone translation of mGBA's singular-matrix object case and
/// compares every LCD pixel against its independently authored expected bitmap.
pub fn verify_degenerate(mut capture: impl FnMut(u64, &[u16])) {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!("../affine-object-degenerate.gba"))
        .unwrap();
    machine
        .advance_to(Cycle(4 * CYCLES_PER_FRAME), 200_000)
        .unwrap();
    assert_eq!(machine.registers()[15], 0x080000dc);
    assert_eq!(machine.inspect16(0x03000000).unwrap(), 0xa3);
    assert_eq!(machine.inspect16(0x03000002).unwrap(), 1);
    assert_eq!(machine.framebuffer_generation(), 4);
    assert!(machine.cycles().0 <= 4 * CYCLES_PER_FRAME + 32);
    let expected = include_bytes!("mgba-expected.bin");
    for (index, bytes) in expected.as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(
            machine.framebuffer()[index],
            u16::from_le_bytes([bytes[0], bytes[1]]),
            "mGBA degenerate pixel=({},{})",
            index % 240,
            index / 240
        );
    }
    capture(4, machine.framebuffer());
    println!(
        "PASS mGBA degenerate: six singular matrices, 38400 pixels, pc=0x080000dc cycles={} instructions={}",
        machine.cycles().0,
        machine.executed_instructions()
    );
}
