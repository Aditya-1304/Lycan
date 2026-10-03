//! Host frame timing shared by fixture commands and bounded diagnostic scenes.

use crate::support::{Result, fail};
use gba_core::{CYCLES_PER_FRAME, Machine};
use std::time::Instant;

/// Shares frame timing between manifest fixtures and bounded interactive diagnostics.
pub(crate) fn benchmark_machine(
    name: &str,
    machine: &mut Machine,
    frames: usize,
    max_instructions: usize,
) -> Result<()> {
    if frames == 0 || frames > 1_000_000 {
        return Err(fail("frame count must be 1..=1000000"));
    }
    let mut pcm = Vec::with_capacity(1024);
    machine.drain_pcm(&mut pcm);
    let mut samples = Vec::with_capacity(frames);
    let mut target = machine.cycles();
    for _ in 0..frames {
        target.0 += CYCLES_PER_FRAME;
        let start = Instant::now();
        machine.advance_to(target, max_instructions)?;
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        pcm.clear();
        machine.drain_pcm(&mut pcm);
    }
    let mean = samples.iter().sum::<f64>() / frames as f64;
    samples.sort_by(f64::total_cmp);
    let p95 = samples[(frames * 95).div_ceil(100).saturating_sub(1)];
    println!(
        "BENCH {} frames={frames} core_mean_ms={mean:.3} core_p95_ms={p95:.3} upload=not-applicable-headless",
        name
    );
    Ok(())
}
