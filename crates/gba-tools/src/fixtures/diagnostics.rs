//! CPU and bus timing diagnostics with explicit guest completion contracts.

use super::manifest::{Diagnostic, Fixture, Timing};
use crate::support::{Result, capture, fail, hash};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine};
use std::path::Path;
use std::time::Instant;

/// Requires exact elapsed cycles, bounded execution, data identity, and the
/// guest's declared mailbox. Reaching a self-branch alone never passes.
pub(super) fn verify_timing(fixture: &Fixture, bytes: &[u8], expected: &Timing) -> Result<()> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    for point in &expected.checkpoints {
        for (pc, measured) in [(point.start_pc, false), (point.end_pc, true)] {
            let before = machine.cycles().0;
            let remaining = fixture
                .max_instructions
                .checked_sub(machine.executed_instructions())
                .ok_or_else(|| fail("timing instruction budget exhausted"))?;
            machine.run_until_pc(pc, remaining, Cycle(fixture.max_cycles))?;
            if machine.inspect16(0x04000204)? != point.waitcnt {
                return Err(fail("guest WAITCNT differs from declared timing case"));
            }
            if measured && machine.cycles().0 - before != point.cycles {
                return Err(fail(format!(
                    "timing WS{} width={} WAITCNT={:#06x}: expected {}, got {}",
                    point.window,
                    point.width,
                    point.waitcnt,
                    point.cycles,
                    machine.cycles().0 - before
                )));
            }
        }
        println!(
            "PASS timing WS{} width={} WAITCNT={:#06x} cycles={}",
            point.window, point.width, point.waitcnt, point.cycles
        );
    }
    let remaining = fixture
        .max_instructions
        .checked_sub(machine.executed_instructions())
        .ok_or_else(|| fail("timing instruction budget exhausted"))?;
    machine.run_until_pc(expected.terminal_pc, remaining, Cycle(fixture.max_cycles))?;
    if machine.inspect16(fixture.mailbox_address)? != fixture.completion_id
        || machine.inspect16(fixture.mailbox_address + 2)? != 1
        || machine.inspect16(fixture.mailbox_address + 4)? != 0x5678
        || machine.inspect16(fixture.mailbox_address + 6)? != 0x1234
    {
        return Err(fail("timing completion/data mailbox mismatch"));
    }
    for (index, point) in expected.checkpoints.iter().enumerate() {
        if u64::from(machine.inspect16(fixture.mailbox_address + 8 + index as u32 * 2)?)
            != point.cycles
        {
            return Err(fail(
                "guest timing report differs from declared expectation",
            ));
        }
    }
    Ok(())
}

/// Runs the pinned guest to its source-defined completion, then verifies the
/// rendered success text after scanout. Failure results are reported by number.
pub(super) fn run_diagnostic(
    fixture: &Fixture,
    bytes: &[u8],
    expected: &Diagnostic,
    capture_path: Option<&str>,
) -> Result<()> {
    if hash(gba_core::TEST_FIRMWARE) != expected.firmware_sha256 {
        return Err(fail("test firmware differs from frozen identity"));
    }
    let offset = expected
        .terminal_pc
        .checked_sub(0x08000000)
        .ok_or_else(|| fail("terminal outside ROM"))? as usize;
    let opcode = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| fail("terminal outside ROM"))?;
    if opcode != expected.terminal_instruction.to_le_bytes() {
        return Err(fail("terminal opcode mismatch"));
    }
    let start = Instant::now();
    let mut machine = Machine::new();
    let backup_override = expected
        .backup_override
        .as_deref()
        .map(str::parse)
        .transpose()?;
    machine.load_rom_with_backup(bytes, backup_override)?;
    machine.enable_test_firmware();
    let report = machine.run_until_pc(
        expected.terminal_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    let completion_ms = start.elapsed().as_secs_f64() * 1000.0;
    let result = machine.registers()[expected.result_register];
    let successful =
        result == expected.result && machine.cpsr() & expected.cpsr_mask == expected.cpsr_value;
    if let Some(digest) = &expected.framebuffer_sha256 {
        let target = Cycle((machine.cycles().0 / CYCLES_PER_FRAME + 2) * CYCLES_PER_FRAME);
        if target.0 > fixture.max_cycles {
            return Err(fail("diagnostic cycle limit leaves no scanout budget"));
        }
        machine.advance_to(
            target,
            fixture.max_instructions.saturating_sub(report.instructions),
        )?;
        let pixels: Vec<u8> = machine
            .framebuffer()
            .iter()
            .flat_map(|p| p.to_le_bytes())
            .collect();
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.{}.ppm", fixture.name)),
                machine.framebuffer(),
            )?;
        }
        if successful && hash(&pixels) != *digest {
            return Err(fail(format!(
                "{} success framebuffer differs: {}",
                fixture.name,
                hash(&pixels)
            )));
        }
    }
    if !successful {
        return Err(fail(format!(
            "{} terminal={:#010x} failed case={} CPSR={:#010x}",
            fixture.name,
            expected.terminal_pc,
            result,
            machine.cpsr()
        )));
    }
    println!(
        "PASS {} terminal={:#010x} r{}={} cpsr={:#010x} cycles={} instructions={} completion_ms={:.3}",
        fixture.name,
        expected.terminal_pc,
        expected.result_register,
        result,
        machine.cpsr(),
        report.cycles.0,
        report.instructions,
        completion_ms
    );
    Ok(())
}
