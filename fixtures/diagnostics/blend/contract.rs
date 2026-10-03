//! Slice-24 guest-driven alpha/brightness/window replay with an
//! independent framebuffer oracle.

use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};

const RED: u16 = 0x001f;
const BLUE: u16 = 0x7c00;
const WHITE: u16 = 0x7fff;
const BLACK: u16 = 0x0000;

fn alpha(first: u16, second: u16, eva: u32, evb: u32) -> u16 {
    let eva = eva.min(16);
    let evb = evb.min(16);

    let r1 = u32::from(first & 31);
    let g1 = u32::from(((first >> 4) & 62) | (first >> 15));
    let b1 = u32::from((first >> 10) & 31);

    let r2 = u32::from(second & 31);
    let g2 = u32::from(((second >> 4) & 62) | (second >> 15));
    let b2 = u32::from((second >> 10) & 31);

    let r = ((r1 * eva + r2 * evb + 8) >> 4).min(31);
    let g = ((g1 * eva + g2 * evb + 8) >> 4).min(63) >> 1;
    let b = ((b1 * eva + b2 * evb + 8) >> 4).min(31);

    (r | (g << 5) | (b << 10)) as u16
}

fn brighten(color: u16, evy: u32) -> u16 {
    let evy = evy.min(16);

    let mut r = u32::from(color & 31);
    let mut g = u32::from(((color >> 4) & 62) | (color >> 15));
    let mut b = u32::from((color >> 10) & 31);

    r += ((31 - r) * evy + 8) >> 4;
    g += ((63 - g) * evy + 8) >> 4;
    b += ((31 - b) * evy + 8) >> 4;

    (r | ((g >> 1) << 5) | (b << 10)) as u16 & 0x7fff
}

fn darken(color: u16, evy: u32) -> u16 {
    let evy = evy.min(16);

    let mut r = u32::from(color & 31);
    let mut g = u32::from(((color >> 4) & 62) | (color >> 15));
    let mut b = u32::from((color >> 10) & 31);

    r -= (r * evy + 7) >> 4;
    g -= (g * evy + 7) >> 4;
    b -= (b * evy + 7) >> 4;

    (r | ((g >> 1) << 5) | (b << 10)) as u16 & 0x7fff
}

fn expected(state: usize, x: usize, y: usize) -> u16 {
    let inside_win0 = (100..160).contains(&x) && (40..120).contains(&y);

    let bg0_opaque = (x + y) & 1 == 0;

    // State 6 removes BG1 only inside WIN0.
    let bg1_enabled = !(state == 6 && inside_win0);

    let base = if bg0_opaque {
        RED
    } else if bg1_enabled {
        BLUE
    } else {
        BLACK
    };

    let background = match state {
        // Regular BG0 -> BG1 alpha.
        1 => {
            if bg0_opaque {
                alpha(RED, BLUE, 8, 8)
            } else {
                BLUE
            }
        }

        // Brighten BG0.
        2 => {
            if bg0_opaque {
                brighten(RED, 8)
            } else {
                BLUE
            }
        }

        // Darken BG0.
        3 => {
            if bg0_opaque {
                darken(RED, 8)
            } else {
                BLUE
            }
        }

        /*
         * Regular alpha is allowed outside WIN0 but its SFX bit is
         * disabled inside WIN0.
         */
        5 => {
            if bg0_opaque && !inside_win0 {
                alpha(RED, BLUE, 8, 8)
            } else if bg0_opaque {
                RED
            } else {
                BLUE
            }
        }

        _ => base,
    };

    /*
     * OBJ0 occupies X=97..104/Y=72..79.
     * Its tile has alternating opaque/transparent horizontal texels.
     */
    let object_opaque = (97..105).contains(&x) && (72..80).contains(&y) && (x - 97) & 1 == 0;

    if !object_opaque {
        return background;
    }

    match state {
        /*
         * Semi OBJ + BLDCNT darken. A valid immediate BG second target
         * forces alpha and suppresses darken.
         */
        4 => {
            if bg0_opaque {
                alpha(WHITE, RED, 8, 8)
            } else {
                alpha(WHITE, BLUE, 8, 8)
            }
        }

        /*
         * Inside WIN0 BG1 is disabled.
         *
         * If BG0 is opaque it is a valid second target -> forced alpha.
         * If BG0 is transparent the immediate lower layer is backdrop,
         * which is not selected as target 2 -> no forced alpha. The
         * ordinary darken fallback is therefore applied to OBJ.
         */
        6 => {
            if bg0_opaque {
                alpha(WHITE, RED, 8, 8)
            } else if inside_win0 {
                darken(WHITE, 8)
            } else {
                alpha(WHITE, BLUE, 8, 8)
            }
        }

        /*
         * WIN0 clears its normal SFX permission, but the semitransparent
         * OBJ still forces alpha when a valid second target exists.
         */
        7 => {
            if bg0_opaque {
                alpha(WHITE, RED, 8, 8)
            } else {
                alpha(WHITE, BLUE, 8, 8)
            }
        }

        // Normal OBJ blocks effects beneath it.
        _ => WHITE,
    }
}

pub fn verify_with_capture(mut capture: impl FnMut(u64, &[u16])) {
    let mut machine = Machine::new();

    machine.load_rom(include_bytes!("../blend.gba")).unwrap();

    let mut frame = 12u64;

    machine
        .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
        .unwrap();

    for state in 0usize..8 {
        if state != 0 {
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
        }

        assert!((0x0800_0000..0x0800_0400).contains(&machine.registers()[15]));

        assert_eq!(machine.framebuffer_generation(), frame);

        assert!(machine.cycles().0 <= frame * CYCLES_PER_FRAME + 32);

        assert_eq!(machine.inspect16(0x0300_0000).unwrap(), 0x00a5);

        assert_eq!(machine.inspect16(0x0300_0002).unwrap(), state as u16);

        for y in 0..160usize {
            for x in 0..240usize {
                assert_eq!(
                    machine.framebuffer()[y * 240 + x],
                    expected(state, x, y),
                    "frame={frame} state={state} x={x} y={y}"
                );
            }
        }

        capture(frame, machine.framebuffer());

        println!(
            "blend frame={frame} state={state} pc={:#010x} \
             cycles={} instructions={} PASS",
            machine.registers()[15],
            machine.cycles().0,
            machine.executed_instructions(),
        );
    }
}
