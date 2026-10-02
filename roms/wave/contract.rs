//! Guest acceptance checks use observable registers, scanout and recorded PCM.
//! The same bounded script executes against native and production WASM cores.
use gba_core::{Button, CYCLES_PER_FRAME, Cycle, Machine};
use gba_session::Resampler;
use std::num::NonZeroU32;
const ROM: &[u8] = include_bytes!("../wave.gba");

/// Measure rising-edge spacing without depending on oscillator implementation.
fn period(signal: &[[f32; 2]], expected: usize) {
    let edges: Vec<_> = signal
        .windows(2)
        .enumerate()
        .filter(|(_, f)| f[0][1] < 0.0 && f[1][1] > 0.0)
        .map(|(i, _)| i)
        .collect();
    assert!(edges.len() > 8, "wave must produce a recorded signal");
    assert!(edges.windows(2).all(|p| p[1] - p[0] == expected));
}

fn render(chunk: u64) -> Vec<[f32; 2]> {
    let mut machine = Machine::new();
    machine.load_rom(ROM).unwrap();
    machine.enable_test_firmware();
    for (frame, button, pressed) in [
        (8, Button::A, true),
        (12, Button::B, true),
        (16, Button::Select, true),
        (20, Button::Up, true),
        (24, Button::Down, true),
        (28, Button::Down, false),
        (32, Button::Up, false),
        (36, Button::Select, false),
        (40, Button::B, false),
        (44, Button::A, false),
    ] {
        machine
            .set_button_at(Cycle(frame * CYCLES_PER_FRAME), button, pressed)
            .unwrap();
    }
    let mut all = Vec::new();
    for frame in 1..=48u64 {
        let mut pcm = Vec::new();
        let target = frame * CYCLES_PER_FRAME;
        while machine.cycles().0 < target {
            machine
                .advance_to(Cycle((machine.cycles().0 + chunk).min(target)), 200_000)
                .unwrap();
            machine.drain_stereo_pcm(&mut pcm);
        }
        assert_eq!(machine.inspect16(0x03000000).unwrap(), 0x77);
        assert_eq!(machine.inspect16(0x03000002).unwrap(), frame as u16);
        assert!(machine.executed_instructions() < 200_000);
        assert_eq!(machine.inspect16(0x04000074).unwrap() & 0xbfff, 0);
        if frame == 26 {
            assert_eq!(machine.inspect16(0x04000084).unwrap(), 0);
            assert!(pcm.iter().all(|f| *f == [0.0; 2]));
        } else if [4, 10, 14, 18, 22, 30, 46].contains(&frame) {
            assert_eq!(machine.inspect16(0x04000084).unwrap(), 0x87);
            assert_eq!(machine.inspect16(0x04000080).unwrap(), 0x3477);
            let isolated: Vec<_> = pcm[64..pcm.len() - 64]
                .iter()
                .map(|f| {
                    let fifo = if (10..=30).contains(&frame) {
                        0.25 * f[0].signum()
                    } else {
                        0.0
                    };
                    [f[0] - fifo, f[1] - fifo]
                })
                .collect();
            assert!(
                isolated.iter().any(|f| f[0] != 0.0),
                "pulse accompaniment remains mixed"
            );
            let expected = if frame == 22 || frame == 30 {
                11.0 / 64.0
            } else {
                15.0 / 64.0
            };
            assert!(
                isolated.iter().all(|f| f[1].abs() == expected),
                "wave gain frame {frame}: {:?}",
                &isolated[..16]
            );
            if frame == 4 || frame == 46 {
                period(&isolated, 4);
            }
            if frame == 10 || frame == 14 {
                period(&isolated, 8);
            }
            if frame == 18 || frame == 22 {
                assert_eq!(machine.inspect16(0x04000070).unwrap() & 0xa0, 0xa0);
            }
            assert_eq!(
                machine.inspect16(0x04000072).unwrap(),
                if frame == 22 || frame == 30 {
                    0x8000
                } else {
                    0x2000
                }
            );
            let bank = machine.inspect16(0x04000070).unwrap() & 0x40 != 0;
            let expected_ram = if bank {
                if frame == 4 || frame == 46 {
                    0xf0f0
                } else {
                    0xff00
                }
            } else {
                0x00ff
            };
            for address in (0x04000090..0x040000a0).step_by(2) {
                assert_eq!(
                    machine.inspect16(address).unwrap(),
                    expected_ram,
                    "opposite bank readback"
                );
            }
        }
        if frame == 4 || frame == 46 {
            for (i, &pixel) in machine.framebuffer().iter().enumerate() {
                let square = (112..128).contains(&(i % 240)) && (72..88).contains(&(i / 240));
                assert_eq!(
                    pixel,
                    if square {
                        if frame == 4 { 31 } else { 992 }
                    } else {
                        0
                    }
                );
            }
        }
        all.extend_from_slice(&pcm);
    }
    let (produced, dropped, empty) = machine.pcm_counters();
    assert_eq!(produced as usize, all.len());
    assert_eq!((dropped, empty), (0, 0));
    assert!(machine.halted());
    assert!((0x08000100..=0x08000110).contains(&machine.registers()[15]));
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "PASS wave chunk={chunk} frames=48 cycles={} instructions={} samples={produced} dropped={dropped} underruns={empty}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    all
}

/// Exact stream equality detects discontinuities introduced by output drains;
/// the production resampler retains history at every supported host rate.
pub fn verify() -> Vec<[f32; 2]> {
    let whole = render(CYCLES_PER_FRAME);
    assert_eq!(whole, render(997));
    for rate in [44_100, 48_000, 96_000] {
        let Some(output_rate) = NonZeroU32::new(rate) else {
            continue;
        };

        for side in 0..2 {
            let mono: Vec<_> = whole.iter().map(|f| f[side]).collect();
            let mut expected = Vec::new();
            Resampler::new(output_rate).process(&mono, &mut expected);
            let mut streamed = Vec::new();
            let mut converter = Resampler::new(output_rate);
            for chunk in mono.chunks(137) {
                converter.process(chunk, &mut streamed);
            }
            assert_eq!(expected, streamed);
        }
    }
    whole
}
