//! Bounded guest acceptance for pulse note timing, controls and streaming audio.
use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};
use gba_session::Resampler;
const ROM: &[u8] = include_bytes!("../pulse.gba");

/// Recorded rising edges independently check the guest's requested frequency.
fn period(signal: &[[f32; 2]], side: usize, frequency: u16) {
    let edges: Vec<_> = signal
        .windows(2)
        .enumerate()
        .filter(|(_, f)| f[0][side] < 0.0 && f[1][side] > 0.0)
        .map(|(i, _)| i)
        .collect();
    assert!(edges.len() >= 2, "missing pulse edges");
    let expected = f32::from(2048 - frequency) / 4.0;
    for edge in edges.windows(2) {
        assert!(
            ((edge[1] - edge[0]) as f32 - expected).abs() <= 1.0,
            "frequency={frequency} expected={expected} measured={}",
            edge[1] - edge[0]
        );
    }
}

fn render(chunk: u64) -> Vec<[f32; 2]> {
    let mut machine = Machine::new();
    machine.load_rom(ROM).unwrap();
    machine.enable_test_firmware();
    for (frame, button, pressed) in [
        (128, Button::A, true),
        (132, Button::B, true),
        (140, Button::B, false),
        (144, Button::Select, true),
        (148, Button::Select, false),
        (152, Button::Up, true),
        (168, Button::Up, false),
        (172, Button::Down, true),
        (176, Button::Down, false),
        (180, Button::A, false),
    ] {
        machine
            .set_button_at(Cycle(frame * CYCLES_PER_FRAME), button, pressed)
            .unwrap();
    }
    let mut all = Vec::new();
    let mut pcm = Vec::new();
    let mut swept = Vec::new();
    let mut envelope_peak = 0.0f32;
    for frame in 1..=184u64 {
        let target = frame * CYCLES_PER_FRAME;
        pcm.clear();
        while machine.cycles().0 < target {
            let next = (machine.cycles().0 + chunk).min(target);
            machine.advance_to(Cycle(next), 200_000).unwrap();
            machine.drain_stereo_pcm(&mut pcm);
        }
        assert_eq!(machine.inspect16(0x03000000).unwrap(), 0x77);
        assert_eq!(machine.inspect16(0x03000002).unwrap(), frame as u16);
        assert!(machine.executed_instructions() < 500_000);
        let note = (((frame - 1) / 32) & 3) as usize;
        assert_eq!(
            machine.inspect16(0x0300000a).unwrap(),
            [1536, 1642, 1707, 1792][note],
            "note timing frame {frame}"
        );
        assert_eq!(
            machine.inspect16(0x0300000c).unwrap(),
            [1024, 1236, 1366, 1536][note],
            "second note timing frame {frame}"
        );
        if [4, 36, 68, 100].contains(&frame) {
            let note = ((frame - 1) / 32) as usize;
            let lead = [1536, 1642, 1707, 1792][note];
            let second = [1024, 1236, 1366, 1536][note];
            assert_eq!(machine.inspect16(0x0300000a).unwrap(), lead);
            assert_eq!(machine.inspect16(0x0300000c).unwrap(), second);
            assert_eq!(machine.inspect16(0x04000084).unwrap(), 0x83);
            period(&pcm, 1, lead);
            period(&pcm, 0, second);
            for (i, &pixel) in machine.framebuffer().iter().enumerate() {
                let square = (112..128).contains(&(i % 240)) && (72..88).contains(&(i / 240));
                assert_eq!(
                    pixel,
                    if square {
                        [31, 992, 31744, 32767][note]
                    } else {
                        0
                    },
                    "pixel {i}"
                );
            }
        }
        if frame == 146 {
            assert_eq!(machine.inspect16(0x04000062).unwrap(), 0xa040);
        }
        if frame == 154 {
            assert_eq!(machine.inspect16(0x04000062).unwrap(), 0xa180);
        }
        if frame == 174 {
            assert_eq!(machine.inspect16(0x04000084).unwrap(), 0);
            assert!(pcm.iter().all(|f| *f == [0.0; 2]));
        }
        if frame == 178 {
            assert_eq!(machine.inspect16(0x04000084).unwrap(), 0x83);
        }
        // FIFO A contributes +/-0.25 to both sides. Its magnitude exceeds the
        // left voice, so the left sign identifies its level without core internals.
        let isolated: Vec<_> = pcm
            .iter()
            .map(|f| [f[0], f[1] - 0.25 * f[0].signum()])
            .collect();
        if frame == 130 {
            let left: Vec<_> = pcm
                .iter()
                .map(|f| [f[0] - 0.25 * f[0].signum(), 0.0])
                .collect();
            period(&isolated, 1, 1536);
            period(&left, 0, 1024);
        }
        if (134..=140).contains(&frame) {
            swept.extend_from_slice(&isolated);
        }
        if frame == 146 {
            let high =
                isolated.iter().filter(|f| f[1] > 0.0).count() as f32 / isolated.len() as f32;
            assert!((0.20..0.30).contains(&high), "25% recorded duty: {high}");
        }
        if frame == 154 {
            envelope_peak = isolated.iter().map(|f| f[1].abs()).fold(0.0, f32::max);
        }
        if frame == 158 {
            let peak = isolated.iter().map(|f| f[1].abs()).fold(0.0, f32::max);
            assert!(peak < envelope_peak, "recorded envelope must decay");
        }
        all.extend_from_slice(&pcm);
    }
    let edges: Vec<_> = swept
        .windows(2)
        .enumerate()
        .filter(|(_, f)| f[0][1] < 0.0 && f[1][1] > 0.0)
        .map(|(i, _)| i)
        .collect();
    let periods: Vec<_> = edges.windows(2).map(|p| p[1] - p[0]).collect();
    assert!(
        periods.iter().any(|&p| p > 400),
        "recorded sweep must lower frequency"
    );
    assert!(
        periods.windows(2).all(|p| p[1] + 1 >= p[0]),
        "decreasing sweep periods must grow"
    );
    let (produced, dropped, empty) = machine.pcm_counters();
    assert_eq!(produced as usize, all.len());
    assert_eq!((dropped, empty), (0, 0));
    assert!(machine.halted());
    assert!((0x080000fc..=0x0800010c).contains(&machine.registers()[15]));
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "PASS pulse chunk={chunk} frames=184 cycles={} instructions={} samples={produced} dropped={dropped} underruns={empty}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    all
}

/// Compare guest execution partitions and the existing production resampler.
pub fn verify() -> Vec<[f32; 2]> {
    let whole = render(CYCLES_PER_FRAME);
    assert_eq!(whole, render(997));
    for rate in [44_100, 48_000, 96_000] {
        for side in 0..2 {
            let mono: Vec<_> = whole.iter().map(|f| f[side]).collect();
            let mut expected = Vec::new();
            Resampler::new(rate).process(&mono, &mut expected);
            let mut converter = Resampler::new(rate);
            let mut streamed = Vec::new();
            for chunk in mono.chunks(137) {
                converter.process(chunk, &mut streamed);
            }
            assert_eq!(expected, streamed);
        }
    }
    whole
}
