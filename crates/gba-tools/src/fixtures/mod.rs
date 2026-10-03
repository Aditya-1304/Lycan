//! Manifest fixture selection, verification dispatch, and scenario benchmarking.
//! Runners are grouped by the guest behavior and oracle they share.

mod audio;
mod build;
mod diagnostics;
mod graphics;
mod interrupts;
mod manifest;
mod pixels;

use self::audio::run_pcm;
use self::diagnostics::{run_diagnostic, verify_timing};
use self::graphics::{
    check_sprite_image, check_tiled_image, run_counter, run_movement, run_palette, run_sprites,
    run_stripes, run_tiled,
};
use self::interrupts::{run_dma, run_keypad, run_vblank};
use self::manifest::{Fixture, Verification, load, manifest};
use self::pixels::{check_image, execute, pixels, prove_store_effect};
use crate::benchmark::benchmark_machine;
use crate::support::{Result, capture, fail, option};
pub(crate) use build::build_fixtures;
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_HEIGHT, SCREEN_WIDTH};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;

/// Measures complete frame advances after scene initialization and replay.
/// Host samples are owned by this CLI; the emulation hot path allocates nothing.
fn benchmark_frames(fixture: &Fixture, machine: &mut Machine, frames: usize) -> Result<()> {
    benchmark_machine(&fixture.name, machine, frames, fixture.max_instructions)
}

/// Executes selected manifest fixtures and retains scenario-specific benchmark checks.
pub(crate) fn run(args: &[String], path: &Path) -> Result<()> {
    let bench = args.first().map(String::as_str) == Some("bench");
    if !args.is_empty()
        && !bench
        && args[0] != "pixels"
        && !(args[0] == "fixtures" && args.get(1).map(String::as_str) == Some("run"))
    {
        return Err(fail(
            "usage: verify-noise [--capture WAV] | verify-wave [--capture WAV] | verify-pulse [--capture WAV] | build-fixtures | fixtures run --manifest PATH [--fixture NAME] [--capture PATH] | bench --scenario pixels|tiled|sprites|vblank|irq-sprites|dma|pcm --frames N",
        ));
    }
    let scenario = option(args, "--scenario")?.unwrap_or_else(|| "pixels".to_owned());
    if bench
        && !matches!(
            scenario.as_str(),
            "pixels" | "tiled" | "sprites" | "vblank" | "irq-sprites" | "dma" | "pcm"
        )
    {
        return Err(fail(
            "benchmark scenario must be pixels, tiled, sprites, vblank, irq-sprites, dma or pcm",
        ));
    }
    let manifest = manifest(path)?;
    let selected = option(args, "--fixture")?;
    if selected
        .as_ref()
        .is_some_and(|name| !manifest.fixture.iter().any(|f| &f.name == name))
    {
        return Err(fail("unknown fixture name"));
    }
    if bench && selected.as_ref().is_some_and(|name| name != &scenario) {
        return Err(fail("benchmark fixture must match its scenario"));
    }
    for fixture in &manifest.fixture {
        if selected.as_ref().is_some_and(|name| &fixture.name != name) {
            continue;
        }
        if bench && fixture.name != scenario {
            continue;
        }
        let directory = path
            .parent()
            .ok_or_else(|| fail("fixture manifest has no parent directory"))?;

        let bytes = load(fixture, directory)?;
        if let Verification::Pcm(expected) = &fixture.verification {
            let mut machine = run_pcm(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames = option(args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
            }
            continue;
        }
        if let Verification::Dma(expected) = &fixture.verification {
            let mut machine = run_dma(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames = option(args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
                if machine.framebuffer().iter().any(|&pixel| pixel != 0x001f) || !machine.halted() {
                    return Err(fail("DMA benchmark scene mismatch"));
                }
            }
            continue;
        }
        if let Verification::Keypad(expected) = &fixture.verification {
            run_keypad(fixture, expected, &bytes)?;
            continue;
        }
        if let Verification::Vblank(expected) = &fixture.verification {
            let mut machine = run_vblank(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames: usize = option(args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                let before = machine.inspect16(fixture.mailbox_address + 2)?;
                benchmark_frames(fixture, &mut machine, frames)?;
                let count = before.wrapping_add(frames as u16);
                if machine.inspect16(fixture.mailbox_address + 2)? != count
                    || !machine.halted()
                    || machine
                        .framebuffer()
                        .iter()
                        .any(|&pixel| pixel != count.wrapping_sub(1) & 31)
                {
                    return Err(fail("VBlank benchmark output mismatch"));
                }
            }
            continue;
        }
        if let Verification::Sprites(expected) = &fixture.verification {
            let mut machine = run_sprites(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            if bench {
                let Some(point) = expected.checkpoints.last() else {
                    return Err(fail("sprite fixture contains no checkpoints"));
                };
                let b = &point.background;
                if usize::from(point.x) != point.image_x
                    || usize::from(point.y) != point.image_y
                    || point.priority != point.image_priority
                    || point.mapping_1d != point.image_mapping_1d
                    || usize::from(b.x) != b.image_x
                    || usize::from(b.y) != b.image_y
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > machine.cycles().0)
                    || gba_session::BUTTONS
                        .iter()
                        .any(|&button| machine.button_pressed(button))
                {
                    return Err(fail("sprite benchmark requires a settled, released replay"));
                }
                let frames: usize = option(args, "--frames")?
                    .unwrap_or_else(|| "120".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
                check_sprite_image(&machine, point)?;
            }
            continue;
        }
        if let Verification::Tiled(expected) = &fixture.verification {
            let mut machine = run_tiled(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames: usize = option(args, "--frames")?
                    .unwrap_or_else(|| "120".to_owned())
                    .parse()?;
                if frames == 0 || frames > 1_000_000 {
                    return Err(fail("frame count must be 1..=1000000"));
                }
                let Some(point) = expected.checkpoints.last() else {
                    return Err(fail("tiled fixture contains no checkpoints"));
                };
                // The replay has released every button. Benchmark stable scanout
                // after initialization and require the same independent image.
                if point.x as usize != point.image_x
                    || point.y as usize != point.image_y
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > machine.cycles().0)
                    || gba_session::BUTTONS
                        .iter()
                        .any(|&button| machine.button_pressed(button))
                {
                    return Err(fail("tiled benchmark requires a settled, released replay"));
                }
                benchmark_frames(fixture, &mut machine, frames)?;
                check_tiled_image(&machine, point.image_x, point.image_y)?;
            }
            continue;
        }
        if let Verification::Stripes(expected) = &fixture.verification {
            if !bench {
                run_stripes(
                    fixture,
                    expected,
                    &bytes,
                    option(args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Diagnostic(expected) = &fixture.verification {
            if !bench {
                run_diagnostic(
                    fixture,
                    &bytes,
                    expected,
                    option(args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Timing(expected) = &fixture.verification {
            verify_timing(fixture, &bytes, expected)?;
            continue;
        }
        if let Verification::Counter(expected) = &fixture.verification {
            if !bench {
                run_counter(
                    fixture,
                    expected,
                    &bytes,
                    option(args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Buttons(expected) = &fixture.verification {
            if bench {
                continue;
            }
            run_movement(
                fixture,
                expected,
                &bytes,
                option(args, "--capture")?.as_deref(),
            )?;
            continue;
        }
        if let Verification::Palette(expected) = &fixture.verification {
            if !bench {
                run_palette(
                    fixture,
                    expected,
                    &bytes,
                    option(args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Hello(expected) = &fixture.verification {
            if !bench {
                let mut machine = Machine::new();
                machine.load_rom(&bytes)?;
                machine.run_until_pc(
                    expected.terminal_pc,
                    fixture.max_instructions,
                    Cycle(fixture.max_cycles),
                )?;
                // Drawing finishes during VBlank, after that frame was published.
                // Wait through a complete subsequent visible frame before capture.
                let next_frame =
                    Cycle((machine.cycles().0 / CYCLES_PER_FRAME + 2) * CYCLES_PER_FRAME);
                if next_frame.0 > fixture.max_cycles {
                    return Err(fail("hello cycle limit leaves no scanout budget"));
                }
                machine.advance_to(next_frame, fixture.max_instructions)?;
                let mut digest = Sha256::new();
                for &pixel in machine.framebuffer() {
                    digest.update(pixel.to_le_bytes());
                }
                let actual = format!("{:x}", digest.finalize());
                if actual != expected.framebuffer_sha256 {
                    return Err(fail(format!(
                        "hello framebuffer SHA-256 {actual}, expected {}",
                        expected.framebuffer_sha256
                    )));
                }
                if let Some(path) = option(args, "--capture")? {
                    capture(
                        Path::new(&format!("{path}.hello.ppm")),
                        machine.framebuffer(),
                    )?;
                }
                println!(
                    "PASS hello terminal={:#010x} cycles={} framebuffer_sha256={actual}",
                    expected.terminal_pc,
                    machine.cycles().0
                );
            }
            continue;
        }
        let expected = pixels(fixture)?;
        let start = Instant::now();
        let mut machine = execute(fixture, &bytes)?;
        let boot_ms = start.elapsed().as_secs_f64() * 1000.0;
        check_image(fixture, &machine)?;
        println!(
            "PASS {} pixels={} terminal={:#010x} cycles={} generation={} execution_ms={boot_ms:.3}",
            fixture.name,
            SCREEN_WIDTH * SCREEN_HEIGHT,
            expected.terminal_pc,
            machine.cycles().0,
            machine.framebuffer_generation()
        );
        if let Some(path) = option(args, "--capture")? {
            capture(Path::new(&path), machine.framebuffer())?;
        }
        if bench {
            let frames: usize = option(args, "--frames")?
                .unwrap_or_else(|| "120".to_owned())
                .parse()?;
            if frames == 0 || frames > 1_000_000 {
                return Err(fail("frame count must be 1..=1000000"));
            }
            let mut samples = Vec::with_capacity(frames);
            let mut target = machine.cycles();
            for _ in 0..frames {
                target.0 += CYCLES_PER_FRAME;
                let start = Instant::now();
                machine.advance_to(target, fixture.max_instructions)?;
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            check_image(fixture, &machine)?;
            let mean = samples.iter().sum::<f64>() / frames as f64;
            samples.sort_by(f64::total_cmp);
            let p95 = samples[(frames * 95).div_ceil(100).saturating_sub(1)];
            println!(
                "BENCH {} frames={frames} core_mean_ms={mean:.3} core_p95_ms={p95:.3} upload=not-applicable-headless",
                fixture.name
            );
        } else if matches!(fixture.verification, Verification::Pixels(_)) {
            prove_store_effect(fixture, &bytes)?;
        }
    }
    Ok(())
}
