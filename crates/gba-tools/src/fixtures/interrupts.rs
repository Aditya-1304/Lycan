//! DMA, keypad, and VBlank guest checkpoints and bounded wake/stall verification.

use super::manifest::{DMA_INPUT, DmaScene, Fixture, KEYPAD_INPUT, Keypad, Vblank};
use crate::support::{Result, capture, fail, hash};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_HEIGHT, SCREEN_WIDTH};
use std::path::Path;

pub(super) fn run_dma(
    fixture: &Fixture,
    expected: &DmaScene,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<Machine> {
    if expected.firmware_sha256 != hash(gba_core::TEST_FIRMWARE) {
        return Err(fail("DMA firmware identity mismatch"));
    }
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.enable_test_firmware();
    machine.run_until_pc(
        expected.ready_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    let initialized = machine.cycles().0;
    if machine.inspect16(fixture.mailbox_address)? != fixture.completion_id {
        return Err(fail("DMA setup mailbox mismatch"));
    }
    for address in (0x06000000..0x06010000).step_by(2) {
        if machine.inspect16(address)?
            != if (0x06008000..0x0600a000).contains(&address) {
                0
            } else {
                0x1111
            }
        {
            return Err(fail("large DMA upload or map clear mismatch"));
        }
    }
    // Input is shared with the app; its frozen frame offsets are checked here.
    for &(cycle, button, pressed) in DMA_INPUT {
        machine.set_button_at(cycle, button, pressed)?;
    }
    for point in &expected.checkpoints {
        let frame = u64::from(point.frame);
        let tile = point.tile;
        let color = point.color;
        let remaining = fixture
            .max_instructions
            .checked_sub(machine.executed_instructions())
            .ok_or_else(|| fail("DMA instruction budget exhausted"))?;
        machine.advance_to(Cycle(frame * CYCLES_PER_FRAME), remaining)?;
        for address in (0x06000000..0x06000020).step_by(2) {
            if machine.inspect16(address)? != tile {
                return Err(fail(format!("DMA tile mismatch at frame {frame}")));
            }
        }
        if machine.framebuffer().iter().any(|&pixel| pixel != color)
            || machine.inspect16(fixture.mailbox_address + 6)? != 0x0801
            || machine.inspect16(fixture.mailbox_address + 8)? != 0
            || !machine.halted()
        {
            return Err(fail(format!(
                "DMA frame/IRQ/HALT mismatch at frame {frame}"
            )));
        }
        let vblank = machine.inspect16(fixture.mailbox_address + 2)?;
        let completed = machine.inspect16(fixture.mailbox_address + 4)?;
        if vblank != point.vblank || completed != point.completions {
            return Err(fail(format!(
                "DMA event order frame={frame} vblank={vblank} completed={completed} setup_cycles={initialized}"
            )));
        }
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.dma-frame-{frame}.ppm")),
                machine.framebuffer(),
            )?;
        }
        println!(
            "PASS dma frame={frame} tile={tile:#06x} color={color:#06x} vblank={vblank} completions={completed} pixels=38400"
        );
    }
    println!(
        "PASS dma setup_cycles={initialized} cycles={} instructions={}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(machine)
}

/// Verifies keypad dispatch, W1C acknowledgement and return to bounded HALT.
pub(super) fn run_keypad(fixture: &Fixture, expected: &Keypad, bytes: &[u8]) -> Result<()> {
    if hash(gba_core::TEST_FIRMWARE) != expected.firmware_sha256 {
        return Err(fail("keypad firmware identity mismatch"));
    }
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.enable_test_firmware();
    for &(cycle, button, pressed) in KEYPAD_INPUT {
        machine.set_button_at(cycle, button, pressed)?;
    }
    let wake = if expected.and_mode { 30_000 } else { 20_000 };
    machine.advance_to(Cycle(wake - 1), fixture.max_instructions)?;
    if machine.inspect16(0x04000202)? != 0
        || machine.inspect16(fixture.mailbox_address + 2)? != 0
        || !machine.halted()
    {
        return Err(fail("unrelated or incomplete keys woke the guest"));
    }
    machine.advance_to(Cycle(wake), fixture.max_instructions)?;
    if machine.inspect16(0x04000202)? != 0x1000
        || machine.inspect16(fixture.mailbox_address + 2)? != 0
        || !machine.halted()
        || machine.cycles().0 != wake
    {
        return Err(fail(
            "keypad requested early, late, or with incorrect IF source",
        ));
    }
    machine.advance_to(Cycle(40_000), fixture.max_instructions)?;
    let count = 1;
    if machine.inspect16(fixture.mailbox_address)? != fixture.completion_id
        || machine.inspect16(fixture.mailbox_address + 2)? != count
        || machine.inspect16(fixture.mailbox_address + 4)? != 0x1000
        || machine.inspect16(fixture.mailbox_address + 6)? != 0
        || machine.inspect16(fixture.mailbox_address + 8)? != count
        || machine.inspect16(0x04000202)? != 0
        || !machine.halted()
        || machine.executed_instructions() > fixture.max_instructions
    {
        return Err(fail(
            "keypad callback, acknowledgement or bounded sleep mismatch",
        ));
    }
    // Mailbox success alone cannot prove that the guest palette store reached
    // scanout. The first frame began before the IRQ; compare every pixel
    // once the second frame has completed entirely after the guest store.
    for frame in 1..=2 {
        let remaining = fixture
            .max_instructions
            .checked_sub(machine.executed_instructions())
            .ok_or_else(|| fail("keypad total instruction budget exhausted"))?;
        machine.advance_to(Cycle(frame * CYCLES_PER_FRAME), remaining)?;
        if machine.framebuffer_generation() != frame
            || machine.framebuffer().len() != SCREEN_WIDTH * SCREEN_HEIGHT
            || (frame == 2 && machine.framebuffer().iter().any(|&pixel| pixel != 0x001f))
        {
            return Err(fail(format!(
                "keypad frame {frame}: expected all 38400 pixels bright red (0x001f)"
            )));
        }
    }
    // A disabled KEYCNT source must not request IF even with matching keys.
    let mut disabled = bytes.to_vec();
    let control = if expected.and_mode { 0x8003u32 } else { 3u32 };
    disabled[0x300..0x304].copy_from_slice(&control.to_le_bytes());
    let mut probe = Machine::new();
    probe.load_rom(&disabled)?;
    probe.enable_test_firmware();
    for &(cycle, button, pressed) in KEYPAD_INPUT {
        probe.set_button_at(cycle, button, pressed)?;
    }
    probe.advance_to(Cycle(40_000), fixture.max_instructions)?;
    if probe.inspect16(0x04000202)? != 0
        || probe.inspect16(fixture.mailbox_address + 2)? != 0
        || probe.inspect16(fixture.mailbox_address + 8)? != 0
        || !probe.halted()
    {
        return Err(fail("disabled keypad source did not remain asleep"));
    }
    println!(
        "PASS {} wake_cycle={wake} IF=0x1000 callbacks={count} cycles={} instructions={} acknowledgement=0 halted=true",
        fixture.name,
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(())
}

/// Checks guest callbacks, scanout, masked wake and disabled-source stall.
pub(super) fn run_vblank(
    fixture: &Fixture,
    expected: &Vblank,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<Machine> {
    if hash(gba_core::TEST_FIRMWARE) != expected.firmware_sha256 {
        return Err(fail("VBlank firmware identity mismatch"));
    }
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.enable_test_firmware();
    for frame in 1..=expected.frames {
        machine.advance_to(
            Cycle(u64::from(frame) * CYCLES_PER_FRAME),
            fixture.max_instructions,
        )?;
        if machine.inspect16(fixture.mailbox_address)? != fixture.completion_id
            || machine.inspect16(fixture.mailbox_address + 2)? != frame
            || machine.inspect16(fixture.mailbox_address + 4)? != 1
            || machine.inspect16(fixture.mailbox_address + 6)? != 0
            || machine.inspect16(fixture.mailbox_address + 8)? != frame
            || !machine.halted()
            || machine.cpsr() & 0xff != 0x5f
            || machine.framebuffer_generation() != u64::from(frame)
            || machine
                .framebuffer()
                .iter()
                .any(|&pixel| pixel != (frame - 1) & 31)
        {
            return Err(fail(format!(
                "VBlank callback/sleep/frame mismatch at frame {frame}"
            )));
        }
    }
    if machine.executed_instructions() >= fixture.max_instructions {
        return Err(fail("VBlank instruction budget exceeded"));
    }
    if let Some(path) = capture_path {
        capture(Path::new(path), machine.framebuffer())?;
    }
    for configuration in [6u32, 5, 3, 15] {
        let mut variant = bytes.to_vec();
        variant
            .get_mut(expected.configuration_offset..expected.configuration_offset + 4)
            .ok_or_else(|| fail("VBlank configuration outside ROM"))?
            .copy_from_slice(&configuration.to_le_bytes());
        let mut probe = Machine::new();
        probe.load_rom(&variant)?;
        probe.enable_test_firmware();
        probe.advance_to(
            Cycle(u64::from(expected.frames) * CYCLES_PER_FRAME),
            fixture.max_instructions,
        )?;
        let wakes = if configuration == 6 {
            0
        } else {
            expected.frames
        };
        if !probe.halted()
            || probe.inspect16(fixture.mailbox_address + 2)?
                != if configuration == 15 {
                    expected.frames
                } else {
                    0
                }
            || probe.inspect16(fixture.mailbox_address + 8)? != wakes
        {
            return Err(fail("VBlank source/mask/Thumb check failed"));
        }
        if configuration == 15 && (!probe.is_thumb() || probe.cpsr() & 0xff != 0x7f) {
            return Err(fail("VBlank IRQ return did not restore Thumb state"));
        }
        println!(
            "PASS vblank configuration={configuration} wakes={wakes} {}",
            if configuration == 6 {
                "expected-stall"
            } else if configuration == 15 {
                "Thumb-exception-return"
            } else {
                "masked-delivery"
            }
        );
    }
    let mut unmapped = Machine::new();
    unmapped.load_rom(bytes)?;
    if unmapped
        .advance_to(Cycle(CYCLES_PER_FRAME), fixture.max_instructions)
        .is_ok()
        || unmapped.inspect16(fixture.mailbox_address + 2)? != 0
    {
        return Err(fail("VBlank callback ran without a mapped exception entry"));
    }
    println!(
        "PASS vblank frames={} cycles={} instructions={} mapped-vector-required",
        expected.frames,
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(machine)
}
