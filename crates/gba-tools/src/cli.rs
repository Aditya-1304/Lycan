//! Command routing for standalone hardware contracts, BIOS probes, and manifest fixtures.

use crate::benchmark::benchmark_machine;
use crate::fixtures::build_fixtures;
use crate::probe::probe_bios;
use crate::support::{Result, capture, fail, hash, option, record_capture_error, root};
use crate::{
    affine_contract, blend_contract, fixtures, noise_contract, object_contract, pulse_contract,
    raster_contract, rtc_contract, wave_contract, window_contract,
};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Routes the process arguments to the matching hardware contract or fixture command.
pub(crate) fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("verify-rtc") {
        let manifest: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/rtc/manifest.toml"),
        )?)?;
        if manifest["sha256"].as_str() != Some(&hash(rtc_contract::ROM)) {
            return Err(fail("RTC ROM differs from frozen identity"));
        }
        let frame = rtc_contract::verify();
        let frame_hash = hash(
            &frame
                .iter()
                .flat_map(|pixel| pixel.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        if manifest["framebuffer_sha256"].as_str() != Some(&frame_hash) {
            return Err(fail("RTC scanout differs from frozen framebuffer identity"));
        }
        println!("RTC framebuffer SHA-256={frame_hash}");
        if let Some(path) = option(&args, "--capture")? {
            capture(Path::new(&path), &frame)?;
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-raster") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/raster/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("fixtures/diagnostics/raster.gba"))?;
        if identity["bytes"].as_integer() != Some(bytes.len() as i64) {
            return Err("Raster ROM size mismatch".into());
        }
        let actual_hash = hash(&bytes);
        if identity["sha256"].as_str() != Some(actual_hash.as_str()) {
            return Err("Raster ROM identity mismatch".into());
        }
        let directory = root().join("target/raster-captures");
        fs::create_dir_all(&directory)?;
        let mut capture_error = None;
        raster_contract::verify_with_capture(|frame, pixels| {
            record_capture_error(
                &mut capture_error,
                directory.join(format!("frame-{frame}.ppm")),
                pixels,
            );
        });
        if let Some(error) = capture_error {
            return Err(error);
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-raster") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../fixtures/diagnostics/raster.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("raster", &mut machine, 600, 2_000_000)?;
        if machine.framebuffer() != image.as_slice()
            || machine.inspect16(0x0300_0000)? != 0x00a6
            || machine.inspect16(0x0300_0002)? != 0
            || machine.inspect16(0x0300_0004)? != 0x0100
        {
            return Err(fail("raster benchmark changed the settled guest contract"));
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-blend") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/blend/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("fixtures/diagnostics/blend.gba"))?;
        if identity["bytes"].as_integer() != Some(bytes.len() as i64) {
            return Err("Blend ROM size mismatch".into());
        }
        let actual_hash = hash(&bytes);
        if identity["sha256"].as_str() != Some(actual_hash.as_str()) {
            return Err("Blend ROM identity mismatch".into());
        }
        let directory = root().join("target/blend-captures");
        fs::create_dir_all(&directory)?;
        let mut capture_error = None;
        blend_contract::verify_with_capture(|frame, pixels| {
            record_capture_error(
                &mut capture_error,
                directory.join(format!("frame-{frame}.ppm")),
                pixels,
            );
        });
        if let Some(error) = capture_error {
            return Err(error);
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-blend") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../fixtures/diagnostics/blend.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("blend", &mut machine, 600, 2_000_000)?;
        if machine.framebuffer() != image.as_slice() || machine.inspect16(0x0300_0000)? != 0x00a5 {
            return Err(fail("blend benchmark changed the settled guest contract"));
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-window") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/window/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("fixtures/diagnostics/window.gba"))?;
        if identity["sha256"].as_str() != Some(hash(&bytes).as_str()) {
            return Err("Window ROM identity mismatch".into());
        }
        let directory = root().join("target/window-captures");
        fs::create_dir_all(&directory)?;
        let mut capture_error = None;
        window_contract::verify_with_capture(|frame, pixels| {
            record_capture_error(
                &mut capture_error,
                directory.join(format!("frame-{frame}.ppm")),
                pixels,
            );
        });
        if let Some(error) = capture_error {
            return Err(error);
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-window") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../fixtures/diagnostics/window.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("window", &mut machine, 600, 2_000_000)?;
        if machine.framebuffer() != image.as_slice() || machine.inspect16(0x0300_0000)? != 0x00a4 {
            return Err(fail("window benchmark changed the settled guest contract"));
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-affine-object") {
        let frames = option(&args, "--frames")?
            .unwrap_or_else(|| "600".to_owned())
            .parse()?;
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!(
            "../../../fixtures/diagnostics/affine-object.gba"
        ))?;
        machine.advance_to(Cycle(20 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("affine-object", &mut machine, frames, 2_000_000)?;
        if machine.framebuffer() != image || machine.inspect16(0x03000000)? != 0xa2 {
            return Err("Affine object benchmark changed the settled scene".into());
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-affine-object") {
        let capture = option(&args, "--capture")?.map(PathBuf::from);
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/affine-object/manifest.toml"),
        )?)?;
        let degenerate =
            fs::read(root().join("fixtures/diagnostics/affine-object-degenerate.gba"))?;
        if identity["mgba_degenerate"]["sha256"].as_str() != Some(hash(&degenerate).as_str()) {
            return Err("mGBA degenerate ROM identity mismatch".into());
        }
        let bytes = fs::read(root().join("fixtures/diagnostics/affine-object.gba"))?;
        if identity["sha256"].as_str() != Some(hash(&bytes).as_str()) {
            return Err("Affine object ROM identity mismatch".into());
        }
        if let Some(directory) = &capture {
            fs::create_dir_all(directory)?;
        }
        let mut capture_error = None;

        let mut capture_image = |frame, pixels: &[u16]| {
            if let Some(directory) = &capture {
                record_capture_error(
                    &mut capture_error,
                    directory.join(format!("frame-{frame}.ppm")),
                    pixels,
                );
            }
        };
        object_contract::verify_with_capture(&mut capture_image);
        object_contract::verify_degenerate(capture_image);
        if let Some(error) = capture_error {
            return Err(error);
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-affine") {
        let capture = option(&args, "--capture")?.map(PathBuf::from);
        if let Some(directory) = &capture {
            fs::create_dir_all(directory)?;
        }
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("fixtures/diagnostics/affine/manifest.toml"),
        )?)?;
        for mode in 1..=5 {
            let bytes =
                fs::read(root().join(format!("fixtures/diagnostics/affine-mode-{mode}.gba")))?;
            if identity[format!("mode_{mode}")]["sha256"].as_str() != Some(hash(&bytes).as_str()) {
                return Err(format!("Affine mode {mode} ROM identity mismatch").into());
            }
        }
        let mut capture_error = None;

        affine_contract::verify_with_capture(|mode, frame, pixels| {
            if let Some(directory) = &capture {
                record_capture_error(
                    &mut capture_error,
                    directory.join(format!("mode-{mode}-frame-{frame}.ppm")),
                    pixels,
                );
            }
        });
        if let Some(error) = capture_error {
            return Err(error);
        }
        return Ok(());
    }
    if matches!(
        args.first().map(String::as_str),
        Some("verify-bios" | "probe")
    ) {
        return probe_bios(&args, args[0] == "verify-bios");
    }
    let path = option(&args, "--manifest")?
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("fixtures/manifest.toml"));
    if args.first().map(String::as_str) == Some("build-fixtures") {
        return build_fixtures(&path);
    }
    if matches!(
        args.first().map(String::as_str),
        Some("verify-pulse" | "verify-wave" | "verify-noise")
    ) {
        let wave = args[0] == "verify-wave";
        let noise = args[0] == "verify-noise";
        #[derive(Deserialize)]
        struct PulseIdentity {
            sha256: String,
        }
        let expected: PulseIdentity =
            toml::from_str(&fs::read_to_string(root().join(if noise {
                "fixtures/diagnostics/noise/manifest.toml"
            } else if wave {
                "fixtures/diagnostics/wave/manifest.toml"
            } else {
                "fixtures/diagnostics/pulse/manifest.toml"
            }))?)?;
        let identity = format!(
            "{:x}",
            Sha256::digest(if noise {
                include_bytes!("../../../fixtures/diagnostics/noise.gba").as_slice()
            } else if wave {
                include_bytes!("../../../fixtures/diagnostics/wave.gba").as_slice()
            } else {
                include_bytes!("../../../fixtures/diagnostics/pulse.gba").as_slice()
            })
        );
        if expected.sha256 != identity {
            return Err(fail("audio fixture ROM differs from frozen identity"));
        }
        let samples = if noise {
            noise_contract::verify()
        } else if wave {
            wave_contract::verify()
        } else {
            pulse_contract::verify()
        };
        if let Some(path) = option(&args, "--capture")? {
            // Export accepted stereo samples in standard PCM16 WAV format.
            let bytes = (samples.len() * 4) as u32;
            let mut wav = Vec::with_capacity(bytes as usize + 44);
            wav.extend_from_slice(b"RIFF");
            wav.extend_from_slice(&(bytes + 36).to_le_bytes());
            wav.extend_from_slice(b"WAVEfmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&2u16.to_le_bytes());
            wav.extend_from_slice(&32768u32.to_le_bytes());
            wav.extend_from_slice(&131072u32.to_le_bytes());
            wav.extend_from_slice(&4u16.to_le_bytes());
            wav.extend_from_slice(&16u16.to_le_bytes());
            wav.extend_from_slice(b"data");
            wav.extend_from_slice(&bytes.to_le_bytes());
            for frame in samples {
                for value in frame {
                    wav.extend_from_slice(&((value * 32767.0) as i16).to_le_bytes());
                }
            }
            fs::write(path, wav)?;
        }
        return Ok(());
    }
    fixtures::run(&args, &path)
}
