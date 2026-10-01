//! Bounded guest acceptance through public machine registers, scanout and PCM.
//! Native and production WASM execute the same inputs and signal checks.
use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};
use gba_session::Resampler;
const ROM: &[u8] = include_bytes!("../noise.gba");
const FRAMES: u64 = 1800;

/// Exercise paired scene events and diagnostic isolation while retaining every
/// oscillator, cascade counter and DMA descriptor across output boundaries.
fn render(chunk: u64) -> Vec<[f32; 2]> {
    let mut machine = Machine::new();
    machine.load_rom(ROM).unwrap();
    machine.enable_test_firmware();
    for (frame, button, pressed) in [
        (8, Button::A, true),
        (12, Button::A, false),
        (16, Button::B, true),
        (20, Button::Select, true),
        (24, Button::Select, false),
        (28, Button::B, false),
        (64, Button::Select, true),
        (96, Button::Select, false),
    ] {
        machine
            .set_button_at(Cycle(frame * CYCLES_PER_FRAME), button, pressed)
            .unwrap();
    }
    let mut all = Vec::new();
    let mut previous_counts: Option<u64> = None;
    for frame in 1..=FRAMES {
        let mut pcm = Vec::new();
        let target = frame * CYCLES_PER_FRAME;
        while machine.cycles().0 < target {
            machine
                .advance_to(Cycle((machine.cycles().0 + chunk).min(target)), 200_000)
                .unwrap();
            machine.drain_stereo_pcm(&mut pcm);
        }
        assert_eq!(machine.inspect16(0x03000000).unwrap(), 0x79);
        assert_eq!(machine.inspect16(0x03000002).unwrap(), frame as u16);
        assert_eq!(machine.inspect16(0x0400007c).unwrap() & 0xb000, 0);
        assert!(
            pcm.iter()
                .all(|f| f.iter().all(|v| v.is_finite() && v.abs() <= 1.0))
        );
        // All four IF flags must latch, even though only VBlank dispatches an
        // IRQ callback. Direct Sound consumes only timers zero and one.
        assert_eq!(machine.inspect16(0x04000202).unwrap() & 0x78, 0x78);
        assert_eq!(machine.inspect16(0x040000c6).unwrap(), 0xb300);
        assert_eq!(machine.inspect16(0x040000d2).unwrap(), 0xb300);
        let counters: Vec<_> = (0..4)
            .map(|i| machine.inspect16(0x04000100 + i * 4).unwrap())
            .collect();
        assert!((0xf800..=0xffff).contains(&counters[0]));
        assert!((0xfffe..=0xffff).contains(&counters[1]));
        assert!((0xfffd..=0xffff).contains(&counters[2]));
        assert!((0xfffc..=0xffff).contains(&counters[3]));
        // Encode the cascaded mixed-radix counter independently of the timer
        // implementation. One frame advances 137 or 138 timer-zero overflows.
        let count = u64::from(counters[0] - 0xf800)
            + 2048
                * (u64::from(counters[1] - 0xfffe)
                    + 2 * (u64::from(counters[2] - 0xfffd) + 3 * u64::from(counters[3] - 0xfffc)));
        if let Some(previous) = previous_counts {
            let delta = (count + 49152 - previous) % 49152;
            assert!(
                delta.abs_diff(CYCLES_PER_FRAME % 49152) <= 32,
                "cascade clock drift: {delta}"
            );
        }
        previous_counts = Some(count);
        if frame == 4 {
            assert_eq!(
                machine.inspect16(0x04000084).unwrap(),
                0x8f,
                "all four PSG voices must be active"
            );
        }
        if frame == 62 || frame == 126 {
            // PCM amplitudes exceed the remaining PSG sum, so the stereo sign
            // independently exposes each FIFO clock after noise length expiry.
            // Frame 62 uses A/timer1 and B/timer0; frame 126 has the same phase.
            for (side, spacing) in [(0, 16), (1, 8)] {
                let edges: Vec<_> = pcm
                    .windows(2)
                    .enumerate()
                    .filter(|(_, pair)| pair[0][side].signum() != pair[1][side].signum())
                    .map(|(index, _)| index)
                    .collect();
                assert!(edges.len() > 16);
                assert!(
                    edges.windows(2).all(|pair| pair[1] - pair[0] == spacing),
                    "FIFO clock selection side {side}"
                );
            }
        }
        if frame == 158 {
            // The opposite paired event restores A/timer0 and B/timer1.
            for (side, spacing) in [(0, 8), (1, 16)] {
                let edges: Vec<_> = pcm
                    .windows(2)
                    .enumerate()
                    .filter(|(_, pair)| pair[0][side].signum() != pair[1][side].signum())
                    .map(|(index, _)| index)
                    .collect();
                assert!(edges.len() > 16);
                assert!(
                    edges.windows(2).all(|pair| pair[1] - pair[0] == spacing),
                    "alternate FIFO clock side {side}"
                );
            }
        }
        if [4, 10, 14, 22, 26, 34, 62, 66, 82, 98].contains(&frame) {
            let latched = machine.inspect16(0x03000004).unwrap();
            let alternate = (latched ^ (latched >> 8)) & 1 != 0;
            let isolated = latched & 4 != 0;
            assert_eq!(
                machine.inspect16(0x04000082).unwrap(),
                if isolated {
                    0x0e
                } else if alternate {
                    0x160e
                } else {
                    0x520e
                }
            );
            assert_eq!(
                machine.inspect16(0x04000080).unwrap(),
                if isolated { 0x8877 } else { 0xbf77 }
            );
            for (i, &pixel) in machine.framebuffer().iter().enumerate() {
                let square = (112..128).contains(&(i % 240)) && (72..88).contains(&(i / 240));
                assert_eq!(
                    pixel,
                    if square {
                        if alternate { 992 } else { 31 }
                    } else {
                        0
                    }
                );
            }
            if frame == 22 {
                assert!(
                    pcm.iter().all(|f| f[0] == f[1]),
                    "isolated noise is stereo centered"
                );
                assert!(pcm.iter().any(|f| f[0] > 0.0));
                assert!(pcm.iter().any(|f| f[0] < 0.0));
            }
            if frame == 62 || frame == 82 {
                assert_eq!(
                    machine.inspect16(0x04000084).unwrap() & 8,
                    0,
                    "length must expire between scene events"
                );
                if isolated {
                    assert!(pcm.iter().all(|f| *f == [0.0; 2]));
                } else {
                    assert!(
                        pcm.iter().any(|f| f[0] != 0.0 && f[1] != 0.0),
                        "other channels continue after noise expires"
                    );
                }
            }
        }
        all.extend_from_slice(&pcm);
    }
    let (produced, dropped, empty) = machine.pcm_counters();
    assert_eq!(produced, FRAMES * CYCLES_PER_FRAME / 512);
    assert_eq!(produced as usize, all.len());
    assert_eq!(
        (dropped, empty),
        (0, 0),
        "both DMA channels must sustain FIFO refill"
    );
    assert!(machine.halted());
    assert!((0x08000198..=0x080001a8).contains(&machine.registers()[15]));
    assert!(machine.executed_instructions() < 200_000);
    // At frame 22, short-mode noise is isolated and its envelope is stable.
    // A 127-edge period at 4096 cycles/edge spans exactly 1016 PCM frames.
    let start = 21 * CYCLES_PER_FRAME as usize / 512;
    assert!(
        all[start..start + 100]
            .iter()
            .zip(&all[start + 1016..])
            .all(|(a, b)| a == b)
    );
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "PASS noise chunk={chunk} frames={FRAMES} cycles={} instructions={} samples={produced} dropped={dropped} underruns={empty}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    all
}

/// Sustained mixing must produce identical streams across drains and maintain
/// an exact, non-drifting output count at each supported host sample rate.
pub fn verify() -> Vec<[f32; 2]> {
    let whole = render(CYCLES_PER_FRAME);
    assert_eq!(whole, render(997));
    for rate in [44_100, 48_000, 96_000] {
        for side in 0..2 {
            let mono: Vec<_> = whole.iter().map(|f| f[side]).collect();
            let mut expected = Vec::new();
            Resampler::new(rate).process(&mono, &mut expected);
            assert_eq!(
                expected.len() as u64,
                ((mono.len() as u64 - 1) * u64::from(rate)).div_ceil(32768)
            );
            let mut streamed = Vec::new();
            let mut converter = Resampler::new(rate);
            for chunk in mono.chunks(137) {
                converter.process(chunk, &mut streamed);
            }
            assert_eq!(expected, streamed);
        }
    }
    whole
}
