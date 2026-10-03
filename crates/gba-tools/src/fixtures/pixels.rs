//! Terminal pixel fixtures and guest-store effects against the independent image oracle.

use super::manifest::{Fixture, Pixels, Verification};
use crate::support::{Result, fail};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_WIDTH};

/// Requires the exact executed terminal PC and both diagnostic RAM values.
pub(super) fn pixels(fixture: &Fixture) -> Result<&Pixels> {
    match &fixture.verification {
        Verification::Pixels(expected)
        | Verification::Calculations(expected)
        | Verification::Copy(expected) => Ok(expected),
        Verification::Buttons(_)
        | Verification::Palette(_)
        | Verification::Hello(_)
        | Verification::Counter(_)
        | Verification::Tiled(_)
        | Verification::Sprites(_)
        | Verification::Stripes(_)
        | Verification::Timing(_)
        | Verification::Diagnostic(_)
        | Verification::Keypad(_)
        | Verification::Vblank(_)
        | Verification::Dma(_)
        | Verification::Pcm(_) => Err(fail("expected a terminal pixels fixture")),
    }
}

pub(super) fn execute(fixture: &Fixture, bytes: &[u8]) -> Result<Machine> {
    let expected = pixels(fixture)?;
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.run_until_pc(
        expected.terminal_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    for check in &expected.memory_checks {
        for offset in 0..check.halfwords {
            let address = check
                .address
                .checked_add(
                    offset
                        .checked_mul(2)
                        .ok_or_else(|| fail("memory expectation overflow"))?,
                )
                .ok_or_else(|| fail("memory expectation overflow"))?;
            if machine.inspect16(address)? != check.value {
                return Err(fail(format!(
                    "{} copied memory mismatch at {address:#010x}",
                    fixture.name
                )));
            }
        }
    }
    let id = machine.inspect16(fixture.mailbox_address)?;
    let result = machine.inspect16(fixture.mailbox_address + 2)?;
    // Diagnostic guests retain the earliest failing case independently of the
    // overall completion result, so a failure identifies the calculation.
    if matches!(fixture.verification, Verification::Calculations(_)) {
        let first_failure = machine.inspect16(fixture.mailbox_address + 4)?;
        if first_failure != 0 {
            return Err(fail(format!(
                "{} first failing case={first_failure}",
                fixture.name
            )));
        }
    }
    if id != fixture.completion_id || result != expected.result {
        return Err(fail(format!(
            "{} completion mailbox mismatch: id={id:#06x}, result={result:#06x}",
            fixture.name
        )));
    }
    if matches!(fixture.verification, Verification::Calculations(_)) {
        println!("PASS calculations mailbox id={id:#06x} result={result} first_failing_case=0");
    }
    // Two frame periods guarantee a complete scanout after the last guest store.
    let target = Cycle(machine.cycles().0 + 2 * CYCLES_PER_FRAME);
    if target.0 > fixture.max_cycles {
        return Err(fail("fixture cycle limit leaves no scanout budget"));
    }
    machine.advance_to(target, fixture.max_instructions)?;
    Ok(machine)
}

pub(super) fn check_image(fixture: &Fixture, machine: &Machine) -> Result<()> {
    let expected = pixels(fixture)?;
    if machine.framebuffer_generation() == 0 {
        return Err(fail("guest produced no completed frame"));
    }
    for (index, &pixel) in machine.framebuffer().iter().enumerate() {
        let expected = expected.colors[(index / SCREEN_WIDTH) / expected.band_height];
        if pixel != expected {
            return Err(fail(format!(
                "{} pixel ({}, {}) expected {expected:#06x}, got {pixel:#06x}",
                fixture.name,
                index % SCREEN_WIDTH,
                index / SCREEN_WIDTH
            )));
        }
    }
    Ok(())
}

/// Proves that the rendered result follows guest stores rather than a host-side pattern.
pub(super) fn prove_store_effect(fixture: &Fixture, bytes: &[u8]) -> Result<()> {
    let mut changed = bytes.to_vec();
    let store = 0xE0C2_30B2u32.to_le_bytes(); // STRH r3,[r2],#2
    let offset = changed
        .as_chunks::<4>()
        .0
        .iter()
        .position(|word| *word == store)
        .ok_or_else(|| fail("fixture contains no expected band store"))?
        * 4;
    // STRH r1,[r2],#2 writes DISPCNT's value into the first band instead of red.
    changed[offset..offset + 4].copy_from_slice(&0xE0C2_10B2u32.to_le_bytes());
    let machine = execute(fixture, &changed)?;
    if machine.framebuffer()[0] != 0x0403 || check_image(fixture, &machine).is_ok() {
        return Err(fail(
            "changing the guest store did not change the framebuffer",
        ));
    }
    println!(
        "PASS {} guest-store mutation changed the first band",
        fixture.name
    );
    Ok(())
}
