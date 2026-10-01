//! Guest-driven full-frame oracle. Expected pixels use source geometry rather
//! than the renderer's tile-addressing or affine-state implementation.
use gba_core::{Button, Cycle, Machine, CYCLES_PER_FRAME};

/// Executes all bounded guest checkpoints and emits only fully verified images.
pub fn verify_with_capture(mut capture: impl FnMut(usize, u64, &[u16])) {
    let roms: [&[u8]; 5] = [
        include_bytes!("../affine-mode-1.gba"),
        include_bytes!("../affine-mode-2.gba"),
        include_bytes!("../affine-mode-3.gba"),
        include_bytes!("../affine-mode-4.gba"),
        include_bytes!("../affine-mode-5.gba"),
    ];
    for (index, rom) in roms.into_iter().enumerate() {
        let mode = index + 1;
        let mut machine = Machine::new();
        machine.load_rom(rom).unwrap();
        let mut frame = 20;
        machine
            .advance_to(Cycle(frame * CYCLES_PER_FRAME), 2_000_000)
            .unwrap();
        let mut angle = 0;
        let mut scale = 1;
        let mut page = false;
        let mut wrap = false;
        for button in [
            None,
            Some(Button::Right),
            Some(Button::B),
            Some(Button::Up),
            Some(Button::A),
            Some(Button::Right),
            Some(Button::Right),
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
                    Button::Right => angle = (angle + 1) % 4,
                    Button::Up => {
                        scale = if scale == 1 { 2 } else { 1 };
                    }
                    Button::A => page = true,
                    Button::B => wrap = true,
                    _ => unreachable!(),
                }
            }
            // Continuous guests must be in their declared VCOUNT polling window,
            // with completed scanout and bounded instruction-boundary overshoot.
            let poll_start = [0x174, 0x170, 0x184, 0x198, 0x190][index] + 0x08000000;
            let pc = machine.registers()[15];
            assert!(
                (poll_start..=poll_start + 20).contains(&pc),
                "mode={mode} unexpected checkpoint PC={pc:#010x}"
            );
            assert_eq!(machine.framebuffer_generation(), frame);
            assert!(machine.cycles().0 <= frame * CYCLES_PER_FRAME + 32);
            assert_eq!(machine.inspect16(0x03000000).unwrap(), 0xa1);
            assert_eq!(machine.inspect16(0x03000002).unwrap(), angle);
            assert_eq!(machine.inspect16(0x03000004).unwrap(), scale * 256);
            assert_eq!(
                machine.inspect16(0x03000006).unwrap(),
                if page { 16 } else { 0 }
            );
            assert_eq!(
                machine.inspect16(0x03000008).unwrap(),
                if wrap { 0x2000 } else { 0 }
            );
            for y in 0..160i32 {
                for x in 0..240i32 {
                    let (dx, dy) = match angle {
                        0 => (x - 120, y - 80),
                        1 => (80 - y, x - 120),
                        2 => (120 - x, 80 - y),
                        3 => (y - 80, 120 - x),
                        _ => unreachable!(),
                    };
                    let (width, height) = match mode {
                        1 | 2 => (128, 128),
                        5 => (160, 128),
                        _ => (240, 160),
                    };
                    let (mut sx, mut sy) = (
                        width / 2 + dx * i32::from(scale),
                        height / 2 + dy * i32::from(scale),
                    );
                    if mode < 3 && wrap {
                        sx = sx.rem_euclid(width);
                        sy = sy.rem_euclid(height);
                    }
                    let expected = if sx < 0 || sy < 0 || sx >= width || sy >= height {
                        0
                    } else if mode < 3 {
                        let color = (sy % 8 * 8 + sx % 8 + 1) as u16;
                        color | color << 5
                    } else if mode == 4 {
                        let color = (((sx / 2 + sy) & 31) + 1) as u16 + if page { 64 } else { 0 };
                        color | color << 5
                    } else {
                        (sx as u16 & 31)
                            | ((sy as u16 & 31) << 5)
                            | if mode == 5 && page { 0x4000 } else { 0 }
                    };
                    assert_eq!(
                        machine.framebuffer()[y as usize * 240 + x as usize],
                        expected,
                        "mode={mode} angle={angle} scale={scale} page={page} wrap={wrap} pixel=({x},{y})"
                    );
                }
            }
            capture(mode, frame, machine.framebuffer());
            println!(
                "PASS affine mode={mode} frame={frame} angle={angle} scale={scale} page={page} wrap={wrap} pc={pc:#010x} cycles={} instructions={}",
                machine.cycles().0,
                machine.executed_instructions()
            );
        }
    }
}
