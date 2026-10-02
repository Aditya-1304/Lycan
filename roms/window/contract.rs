//! Guest-driven window replay with an independent screen-geometry oracle.
use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};

/// Catches layer leakage, wrong overlap precedence, opaque object-window holes,
/// and mosaic sampling after masking. Existing affine captures exercise none of these.
pub fn verify_with_capture(mut capture: impl FnMut(u64, &[u16])) {
    let mut machine = Machine::new();
    machine.load_rom(include_bytes!("../window.gba")).unwrap();
    let mut state = [0u16; 4];
    let mut frame = 12;
    machine
        .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
        .unwrap();
    for button in [
        None,
        Some(Button::Right),
        Some(Button::A),
        Some(Button::B),
        Some(Button::Right),
        Some(Button::Right),
        Some(Button::Left),
        Some(Button::B),
        Some(Button::A),
        Some(Button::Up),
        Some(Button::Down),
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
                Button::Right => state[0] = (state[0] + 16) & 112,
                Button::Left => state[0] = state[0].wrapping_sub(16) & 112,
                Button::A => state[1] ^= 1,
                Button::B => state[2] ^= 1,
                Button::Up => state[3] = 1,
                Button::Down => state[3] = 2,
                _ => unreachable!(),
            }
        }
        assert!((0x080001b0..=0x080001c4).contains(&machine.registers()[15]));
        assert_eq!(machine.framebuffer_generation(), frame);
        assert!(machine.cycles().0 <= frame * CYCLES_PER_FRAME + 32);
        assert_eq!(machine.inspect16(0x03000000).unwrap(), 0xa4);
        for (slot, value) in state.iter().enumerate() {
            assert_eq!(
                machine.inspect16(0x03000002 + slot as u32 * 2).unwrap(),
                *value
            );
        }
        for y in 0..160usize {
            for x in 0..240usize {
                let inside_first = (match state[3] {
                    0 => (40 + state[0] as usize..80 + state[0] as usize).contains(&x),
                    1 => !(20..220).contains(&x),
                    2 => false,
                    _ => unreachable!(),
                }) && (40..80).contains(&y);
                let inside_second = (70..110).contains(&x) && (60..100).contains(&y);
                let object_window =
                    (96..104).contains(&x) && (56..64).contains(&y) && (x - 96 + y - 56) % 2 == 0;
                let mask = if inside_first {
                    if state[1] == 0 { 0x33 } else { 0x32 }
                } else if inside_second {
                    2
                } else if object_window {
                    1
                } else {
                    0x12
                };
                let sample_x = if state[2] != 0 { x / 4 * 4 } else { x };
                let sample_y = if state[2] != 0 { y / 4 * 4 } else { y };
                let mut expected = if mask & 1 != 0 {
                    if (sample_x + sample_y) % 2 == 0 {
                        31
                    } else {
                        0x7c00
                    }
                } else if mask & 2 != 0 {
                    0x3e0
                } else {
                    0
                };
                let local_x = x.wrapping_sub(100);
                let local_y = y.wrapping_sub(60);
                let object_x = if state[2] != 0 {
                    local_x / 3 * 3
                } else {
                    local_x
                };
                let object_y = if state[2] != 0 {
                    local_y / 3 * 3
                } else {
                    local_y
                };
                if mask & 0x10 != 0 && local_x < 8 && local_y < 8 && (object_x + object_y) % 2 == 0
                {
                    expected = 0x7fff;
                }
                assert_eq!(
                    machine.framebuffer()[y * 240 + x],
                    expected,
                    "frame {frame}, ({x},{y}), state {state:?}"
                );
            }
        }
        capture(frame, machine.framebuffer());
        println!(
            "window frame={frame} pc={:#x} cycles={} instructions={} state={state:?} PASS",
            machine.registers()[15],
            machine.cycles().0,
            machine.executed_instructions()
        );
    }
}
