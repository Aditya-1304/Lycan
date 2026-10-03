//! PCM and mixer replay checkpoints, digests, and stereo captures.

use super::manifest::{Fixture, MIXER_INPUT, PCM_INPUT, PcmScene};
use crate::support::{Result, capture, fail, hash};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_WIDTH};
use std::fs;
use std::path::Path;

/// Verifies bus ownership during setup and the two-frame input/upload/scanout
/// pipeline. Full-frame colors and all copied tile words are independent oracles.
/// Executes the same guest used by the app. Every frame drains core PCM so the
/// hash covers real FIFO output without depending on the staging capacity.
pub(super) fn run_pcm(
    fixture: &Fixture,
    expected: &PcmScene,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<Machine> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.enable_test_firmware();
    machine.run_until_pc(
        expected.ready_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    let setup = machine.cycles().0;
    if machine.inspect16(fixture.mailbox_address)? != fixture.completion_id {
        return Err(fail("PCM setup mailbox mismatch"));
    }
    for &(cycle, button, pressed) in PCM_INPUT {
        machine.set_button_at(cycle, button, pressed)?;
    }
    let mut pcm = Vec::with_capacity(1024);
    let last = expected
        .checkpoints
        .last()
        .ok_or_else(|| fail("missing PCM checkpoint"))?
        .frame;
    let mut total = 0;
    for frame in 1..=last {
        let remaining = fixture
            .max_instructions
            .checked_sub(machine.executed_instructions())
            .ok_or_else(|| fail("PCM instruction budget exhausted"))?;
        machine.advance_to(Cycle(u64::from(frame) * CYCLES_PER_FRAME), remaining)?;
        pcm.clear();
        machine.drain_pcm(&mut pcm);
        total += pcm.len();
        if let Some(point) = expected
            .checkpoints
            .iter()
            .find(|point| point.frame == frame)
        {
            let encoded: Vec<u8> = pcm
                .iter()
                .map(|sample| (sample * 128.0) as i8 as u8)
                .collect();
            let identity = hash(&encoded);
            if let Some(path) = capture_path {
                fs::write(format!("{path}.pcm-frame-{frame}.s8"), &encoded)?;
                capture(
                    Path::new(&format!("{path}.pcm-frame-{frame}.ppm")),
                    machine.framebuffer(),
                )?;
            }
            if pcm.len() != point.samples || identity != point.pcm_sha256 {
                return Err(fail(format!(
                    "PCM frame={frame} samples={} sha256={identity}",
                    pcm.len()
                )));
            }
            // The fixture's independent waveform is silence or alternating
            // +32/-32 bytes, each held for eight core samples. Interior runs
            // must be exact; the frame's first and last runs may be partial.
            if point.pressed == 0 {
                if encoded.iter().any(|&sample| sample != 0) {
                    return Err(fail("idle PCM is not silent"));
                }
            } else {
                if encoded.iter().any(|&sample| !matches!(sample, 32 | 224)) {
                    return Err(fail("active PCM has an unexpected level"));
                }
                let mut start = 0;
                for end in 1..encoded.len() {
                    if encoded[end] != encoded[end - 1] {
                        if start != 0 && end - start != 8 {
                            return Err(fail("PCM waveform period mismatch"));
                        }
                        start = end;
                    }
                }
            }
            for (index, &pixel) in machine.framebuffer().iter().enumerate() {
                let (x, y) = (index % SCREEN_WIDTH, index / SCREEN_WIDTH);
                let color = if (112..128).contains(&x) && (72..88).contains(&y) {
                    point.color
                } else {
                    0
                };
                if pixel != color {
                    return Err(fail(format!("PCM square frame={frame} pixel={index}")));
                }
            }
            if machine.inspect16(fixture.mailbox_address + 2)? != frame
                || machine.inspect16(fixture.mailbox_address + 4)? != point.pressed
                || machine.inspect16(fixture.mailbox_address + 6)? != point.transitions
                || machine.inspect16(0x04000202)? != 0
                || !machine.halted()
            {
                return Err(fail(format!("PCM guest event mismatch frame={frame}")));
            }
            println!(
                "PASS pcm frame={frame} samples={} sha256={identity} square={:#06x} transitions={} pixels=38400",
                pcm.len(),
                point.color,
                point.transitions
            );
        }
    }
    let (produced, dropped, empty) = machine.pcm_counters();
    if produced != machine.cycles().0 / 512
        || produced != total as u64
        || dropped != 0
        || empty != 0
    {
        return Err(fail(format!(
            "PCM production mismatch produced={produced} total={total} dropped={dropped} empty={empty}"
        )));
    }
    println!(
        "PASS pcm setup_cycles={setup} cycles={} instructions={} produced={produced} dropped={dropped} fifo_underruns={empty} rate=32768",
        machine.cycles().0,
        machine.executed_instructions()
    );
    run_mixer(&mut machine, fixture, expected, capture_path)?;
    Ok(machine)
}

/// Continues the original ROM through guest-written mixer settings. Expected
/// stereo levels are independently derived from the signed FIFO bytes, gain,
/// routing and DAC bias; captures retain both channels without downmixing.
pub(super) fn run_mixer(
    machine: &mut Machine,
    fixture: &Fixture,
    expected: &PcmScene,
    capture_path: Option<&str>,
) -> Result<()> {
    for &(cycle, button, pressed) in MIXER_INPUT {
        machine.set_button_at(cycle, button, pressed)?;
    }
    let last = expected
        .mixer_checkpoints
        .last()
        .ok_or_else(|| fail("missing mixer checkpoint"))?
        .frame;
    let first = expected
        .checkpoints
        .last()
        .ok_or_else(|| fail("missing PCM checkpoint"))?
        .frame
        + 1;
    let mut pcm = Vec::with_capacity(1024);
    for frame in first..=last {
        let remaining = fixture
            .max_instructions
            .checked_sub(machine.executed_instructions())
            .ok_or_else(|| fail("mixer instruction budget exhausted"))?;
        machine.advance_to(Cycle(u64::from(frame) * CYCLES_PER_FRAME), remaining)?;
        pcm.clear();
        machine.drain_stereo_pcm(&mut pcm);
        if let Some(point) = expected.mixer_checkpoints.iter().find(|p| p.frame == frame) {
            let levels: Vec<[i16; 2]> = pcm.iter().map(|f| f.map(|v| (v * 512.0) as i16)).collect();
            if levels.iter().any(|f| !point.levels.contains(f))
                || point.levels.iter().any(|f| !levels.contains(f))
            {
                return Err(fail(format!(
                    "mixer levels mismatch frame={frame} observed={:?}",
                    levels
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                )));
            }
            let encoded: Vec<u8> = levels
                .iter()
                .flat_map(|f| f.iter().flat_map(|v| v.to_le_bytes()))
                .collect();
            let identity = hash(&encoded);
            if let Some(path) = capture_path {
                fs::write(format!("{path}.mixer-frame-{frame}.s16le"), &encoded)?;
            }
            if identity != point.pcm_sha256 {
                return Err(fail(format!("mixer frame={frame} sha256={identity}")));
            }
            if machine.inspect16(0x04000082)? != point.control_h
                || machine.inspect16(0x04000084)? != point.master
                || machine.inspect16(0x04000088)? != point.bias
                || machine.inspect16(fixture.mailbox_address + 2)? != frame
                || machine.inspect16(fixture.mailbox_address + 4)? != 1
                || machine.inspect16(fixture.mailbox_address + 6)? != 3
                || !machine.halted()
            {
                return Err(fail(format!("mixer register/guest mismatch frame={frame}")));
            }
            println!(
                "PASS mixer frame={frame} samples={} sha256={identity} control_h={:#06x} master={:#04x} bias={:#06x}",
                pcm.len(),
                point.control_h,
                point.master,
                point.bias
            );
        }
    }
    let (produced, dropped, empty) = machine.pcm_counters();
    if produced != machine.cycles().0 / 512
        || dropped != 0
        || empty != expected.mixer_fifo_underruns
    {
        return Err(fail("mixer sample production mismatch"));
    }
    println!(
        "PASS mixer cycles={} instructions={} produced={produced} dropped={dropped} fifo_underruns={empty}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(())
}
