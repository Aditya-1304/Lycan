//! Input replays and full-frame oracles for movement, palettes, tiles, sprites, and references.

use super::manifest::{
    Counter, DEMO_INPUT, Fixture, Movement, Palette, SpriteCheckpoint, Sprites, Stripes, Tiled,
};
use crate::support::{Result, button, capture, fail, hash};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_HEIGHT, SCREEN_WIDTH};
use gba_session::Session;
use std::path::Path;

/// Executes the original square guest through the shared session. Every checkpoint
/// verifies a declared mailbox and all pixels, including erased square positions.
pub(super) fn run_movement(
    fixture: &Fixture,
    expected: &Movement,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<()> {
    let script: Vec<_> = expected
        .input_events
        .iter()
        .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
        .collect::<Result<_>>()?;
    if script.as_slice() != DEMO_INPUT {
        return Err(fail(
            "app replay input differs from the frozen fixture script",
        ));
    }
    let mut session = Session::new();
    session.load_rom(bytes)?;
    for event in &expected.input_events {
        session.set_button_at(Cycle(event.cycle), button(&event.button)?, event.pressed)?;
    }
    let mut frame = 0;
    for checkpoint in &expected.checkpoints {
        while frame < checkpoint.frame {
            session.advance_frame_with_budget(fixture.max_instructions)?;
            frame += 1;
            if session.cycles().0 > fixture.max_cycles {
                return Err(fail("movement fixture exceeded its cycle limit"));
            }
        }
        let actual = [
            session.inspect16(fixture.mailbox_address)?,
            session.inspect16(fixture.mailbox_address + 2)?,
            session.inspect16(fixture.mailbox_address + 4)?,
            session.inspect16(fixture.mailbox_address + 6)?,
        ];
        let wanted = [
            fixture.completion_id,
            checkpoint.x,
            checkpoint.y,
            checkpoint.frame,
        ];
        if actual != wanted || session.framebuffer_generation() != u64::from(checkpoint.frame) {
            return Err(fail(format!(
                "{} frame {} mailbox {actual:?}, expected {wanted:?}",
                fixture.name, checkpoint.frame
            )));
        }
        for (index, &pixel) in session.framebuffer().iter().enumerate() {
            let x = index % SCREEN_WIDTH;
            let y = index / SCREEN_WIDTH;
            let inside = (checkpoint.image_x..checkpoint.image_x + expected.square_size)
                .contains(&x)
                && (checkpoint.image_y..checkpoint.image_y + expected.square_size).contains(&y);
            let wanted = if inside { expected.color } else { 0 };
            if pixel != wanted {
                return Err(fail(format!(
                    "{} frame {} pixel ({x},{y}) expected {wanted:#06x}, got {pixel:#06x}",
                    fixture.name, checkpoint.frame
                )));
            }
        }
        println!(
            "PASS {} frame={} guest=({},{}) scanout=({},{}) pixels={}",
            fixture.name,
            checkpoint.frame,
            checkpoint.x,
            checkpoint.y,
            checkpoint.image_x,
            checkpoint.image_y,
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
    }
    if let Some(path) = capture_path {
        capture(Path::new(path), session.framebuffer())?;
    }
    Ok(())
}

/// Checks complete frames after each guest-controlled transition. The capture
/// suffix preserves all three outputs rather than overwriting the earlier pages.
pub(super) fn run_palette(
    fixture: &Fixture,
    expected: &Palette,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<()> {
    let mut session = Session::new();
    session.load_rom(bytes)?;
    for event in &expected.input_events {
        session.set_button_at(Cycle(event.cycle), button(&event.button)?, event.pressed)?;
    }
    let mut frame = 0;
    for point in &expected.checkpoints {
        while frame < point.frame {
            session.advance_frame_with_budget(fixture.max_instructions)?;
            frame += 1;
            if session.cycles().0 > fixture.max_cycles {
                return Err(fail("palette fixture exceeded its cycle limit"));
            }
        }
        let actual = [
            session.inspect16(fixture.mailbox_address)?,
            session.inspect16(fixture.mailbox_address + 2)?,
            session.inspect16(fixture.mailbox_address + 4)?,
        ];
        let wanted = [fixture.completion_id, point.page, point.palette_changed];
        if actual != wanted || session.framebuffer_generation() != u64::from(point.frame) {
            return Err(fail(format!(
                "{} frame {} mailbox {actual:?}, expected {wanted:?}",
                fixture.name, point.frame
            )));
        }
        for (index, &pixel) in session.framebuffer().iter().enumerate() {
            let wanted = point.colors[index % 2];
            if pixel != wanted {
                return Err(fail(format!(
                    "{} frame {} pixel ({},{}) expected {wanted:#06x}, got {pixel:#06x}",
                    fixture.name,
                    point.frame,
                    index % SCREEN_WIDTH,
                    index / SCREEN_WIDTH
                )));
            }
        }
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.palette-frame-{}.ppm", point.frame)),
                session.framebuffer(),
            )?;
        }
        println!(
            "PASS {} frame={} page={} palette_changed={} pixels={}",
            fixture.name,
            point.frame,
            point.page,
            point.palette_changed,
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
    }
    Ok(())
}

/// Samples the independent procedural scene after bounded guest execution.
/// The oracle uses world coordinates, not emulated VRAM or renderer helpers.
pub(super) fn tiled_pixel(index: usize, scroll_x: usize, scroll_y: usize) -> u16 {
    let x = (index % SCREEN_WIDTH + scroll_x) % 512;
    let y = (index / SCREEN_WIDTH + scroll_y) % 512;
    let tx = x / 8;
    let ty = y / 8;
    let local_x = if tx.is_multiple_of(2) {
        x % 8
    } else {
        7 - x % 8
    };
    let local_y = if ty.is_multiple_of(2) {
        y % 8
    } else {
        7 - y % 8
    };
    let color = if (tx + ty).is_multiple_of(2) {
        local_x
    } else {
        local_y
    };
    let bank = x / 256 + 2 * (y / 256);
    if color == 0 {
        0x7fff
    } else {
        ((bank + 1) * 256 + color * 33) as u16
    }
}

pub(super) fn check_tiled_image(machine: &Machine, scroll_x: usize, scroll_y: usize) -> Result<()> {
    for (index, &actual) in machine.framebuffer().iter().enumerate() {
        let wanted = tiled_pixel(index, scroll_x, scroll_y);
        if actual != wanted {
            return Err(fail(format!(
                "tiled pixel ({},{}) scroll=({scroll_x},{scroll_y}) expected {wanted:#06x}, got {actual:#06x}",
                index % SCREEN_WIDTH,
                index / SCREEN_WIDTH
            )));
        }
    }
    Ok(())
}

/// The oracle describes the two guest shapes directly in screen coordinates.
/// OAM0 owns opaque overlaps even when BG0 hides it; OAM1 must not leak through.
pub(super) fn check_sprite_image(machine: &Machine, point: &SpriteCheckpoint) -> Result<()> {
    let b = &point.background;
    for (index, &actual) in machine.framebuffer().iter().enumerate() {
        let x = index % SCREEN_WIDTH;
        let y = index / SCREEN_WIDTH;
        let backdrop = tiled_pixel(index, b.image_x, b.image_y);
        let player = if x > point.image_x
            && x < point.image_x + 15
            && y > point.image_y
            && y < point.image_y + 15
        {
            let quadrant =
                usize::from(x >= point.image_x + 8) + 2 * usize::from(y >= point.image_y + 8);
            Some((
                if point.image_mapping_1d {
                    [0x001f, 0x03e0, 0x7c00, 0x03ff][quadrant]
                } else {
                    [0x001f, 0x03e0, 0x03ff, 0x7c00][quadrant]
                },
                point.image_priority,
            ))
        } else {
            None
        };
        let object = player
            .or_else(|| ((120..136).contains(&x) && (80..96).contains(&y)).then_some((0x7c1f, 0)));
        let wanted = match object {
            Some((color, priority)) if priority == 0 || backdrop == 0x7fff => color,
            _ => backdrop,
        };
        if actual != wanted {
            return Err(fail(format!(
                "sprites frame {} pixel ({x},{y}) expected {wanted:#06x}, got {actual:#06x}",
                b.frame
            )));
        }
    }
    Ok(())
}

/// One bounded replay checks guest RAM, OAM, scroll registers and full scanout.
pub(super) fn run_sprites(
    fixture: &Fixture,
    expected: &Sprites,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<Machine> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    if let Some(expected) = &expected.firmware_sha256 {
        if hash(gba_core::TEST_FIRMWARE) != *expected {
            return Err(fail("IRQ sprite firmware identity mismatch"));
        }
        machine.enable_test_firmware();
    }

    for event in &expected.input_events {
        machine.set_button_at(Cycle(event.cycle), button(&event.button)?, event.pressed)?;
    }
    machine.run_until_pc(
        expected.ready_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    println!("sprites initialization cycles={}", machine.cycles().0);
    for point in &expected.checkpoints {
        let b = &point.background;
        machine.advance_to(
            Cycle(u64::from(b.frame) * CYCLES_PER_FRAME),
            fixture
                .max_instructions
                .saturating_sub(machine.executed_instructions()),
        )?;
        if machine.cycles().0 > fixture.max_cycles {
            return Err(fail("sprites exceeded cycle budget"));
        }
        let wanted = [
            fixture.completion_id,
            b.x,
            b.y,
            b.updates,
            point.x,
            point.y,
            point.priority,
            u16::from(point.mapping_1d),
        ];
        for (slot, wanted) in wanted.into_iter().enumerate() {
            let actual = machine.inspect16(fixture.mailbox_address + slot as u32 * 2)?;
            if actual != wanted {
                return Err(fail(format!(
                    "sprites frame {} mailbox slot {slot}: expected {wanted}, got {actual}",
                    b.frame
                )));
            }
        }
        if machine.inspect16(0x07000000)? != point.y
            || machine.inspect16(0x07000002)? != (0x4000 | point.x)
            || machine.inspect16(0x07000004)? != (0x3004 | (point.priority << 10))
            || machine.inspect16(0x04000000)? != (0x1100 | (u16::from(point.mapping_1d) << 6))
            || machine.inspect16(0x04000010)? != b.x
            || machine.inspect16(0x04000012)? != b.y
            || machine.framebuffer_generation() != u64::from(b.frame)
        {
            return Err(fail("sprite OAM/register/frame state mismatch"));
        }
        if expected.firmware_sha256.is_some()
            && (!machine.halted()
                || machine.inspect16(fixture.mailbox_address + 16)? != b.updates
                || machine.inspect16(fixture.mailbox_address + 20)? != 1
                || machine.inspect16(fixture.mailbox_address + 22)? != 0)
        {
            return Err(fail(
                "IRQ sprite scene did not acknowledge and sleep between updates",
            ));
        }
        check_sprite_image(&machine, point)?;
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.sprites-frame-{}.ppm", b.frame)),
                machine.framebuffer(),
            )?;
        }
        println!(
            "PASS sprites frame={} player=({},{}) image=({},{}) priority={}/{} mapping_1d={}/{} pixels={}",
            b.frame,
            point.x,
            point.y,
            point.image_x,
            point.image_y,
            point.priority,
            point.image_priority,
            point.mapping_1d,
            point.image_mapping_1d,
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
    }
    println!(
        "PASS sprites cycles={} instructions={}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(machine)
}

/// Checks the setup PC, guest mailbox, scroll registers, and published frames
/// under one shared instruction/cycle budget for the entire replay.
pub(super) fn run_tiled(
    fixture: &Fixture,
    expected: &Tiled,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<Machine> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    for event in &expected.input_events {
        machine.set_button_at(Cycle(event.cycle), button(&event.button)?, event.pressed)?;
    }
    machine.run_until_pc(
        expected.ready_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    println!("tiled initialization cycles={}", machine.cycles().0);
    for point in &expected.checkpoints {
        machine.advance_to(
            Cycle(u64::from(point.frame) * CYCLES_PER_FRAME),
            fixture
                .max_instructions
                .saturating_sub(machine.executed_instructions()),
        )?;
        if machine.cycles().0 > fixture.max_cycles {
            return Err(fail("tiled exceeded cycle budget"));
        }
        let actual = [
            machine.inspect16(fixture.mailbox_address)?,
            machine.inspect16(fixture.mailbox_address + 2)?,
            machine.inspect16(fixture.mailbox_address + 4)?,
            machine.inspect16(fixture.mailbox_address + 6)?,
        ];
        let updates = point.updates;
        if actual != [fixture.completion_id, point.x, point.y, updates]
            || machine.inspect16(0x04000010)? != point.x
            || machine.inspect16(0x04000012)? != point.y
            || machine.framebuffer_generation() != u64::from(point.frame)
        {
            return Err(fail(format!(
                "tiled frame {} unexpected mailbox {actual:?}, expected updates={updates}",
                point.frame
            )));
        }
        check_tiled_image(&machine, point.image_x, point.image_y)?;
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.tiled-frame-{}.ppm", point.frame)),
                machine.framebuffer(),
            )?;
        }
        println!(
            "PASS tiled frame={} scroll=({},{}) image=({},{}) pixels={}",
            point.frame,
            point.x,
            point.y,
            point.image_x,
            point.image_y,
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
    }
    println!(
        "PASS tiled cycles={} instructions={}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(machine)
}

/// Executes the pinned reference to its declared idle branch, then compares
/// every pixel against the alternating colors specified by upstream source.
pub(super) fn run_stripes(
    fixture: &Fixture,
    expected: &Stripes,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<()> {
    let mut machine = Machine::new();
    // The pinned 324-byte ROM ends at its idle branch. Supply two unexecuted
    // look-ahead words for the current bounded-ROM loader; retain the upstream
    // hash over the original bytes. This does not emulate absent-ROM bus reads.
    let mut mapped = bytes.to_vec();
    mapped.extend_from_slice(&[0; 8]);
    machine.load_rom(&mapped)?;
    machine.run_until_pc(
        expected.terminal_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    let opcode = u32::from(machine.inspect16(expected.terminal_pc)?)
        | (u32::from(machine.inspect16(expected.terminal_pc + 2)?) << 16);
    if opcode != 0xeafffffe {
        return Err(fail("stripes source-defined idle instruction differs"));
    }
    let target = (machine.cycles().0 / CYCLES_PER_FRAME + 2) * CYCLES_PER_FRAME;
    if target > fixture.max_cycles {
        return Err(fail("stripes has no scanout budget"));
    }
    machine.advance_to(
        Cycle(target),
        fixture
            .max_instructions
            .saturating_sub(machine.executed_instructions()),
    )?;
    for (index, &actual) in machine.framebuffer().iter().enumerate() {
        let wanted = if (index % SCREEN_WIDTH / 8).is_multiple_of(2) {
            0x560b
        } else {
            0x6290
        };
        if actual != wanted {
            return Err(fail(format!(
                "stripes pixel {index} expected {wanted:#06x}, got {actual:#06x}"
            )));
        }
    }
    if let Some(path) = capture_path {
        capture(
            Path::new(&format!("{path}.stripes.ppm")),
            machine.framebuffer(),
        )?;
    }
    println!(
        "PASS stripes terminal={:#010x} cycles={} pixels={}",
        expected.terminal_pc,
        machine.cycles().0,
        SCREEN_WIDTH * SCREEN_HEIGHT
    );
    Ok(())
}

/// Executes every declared ARM return before sampling RAM and completed scanout.
/// The manifest supplies independent expected values; a looping ROM is not success.
pub(super) fn run_counter(
    fixture: &Fixture,
    expected: &Counter,
    bytes: &[u8],
    capture_path: Option<&str>,
) -> Result<()> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    for event in &expected.input_events {
        machine.set_button_at(Cycle(event.cycle), button(&event.button)?, event.pressed)?;
    }
    let mut frame = 0;
    for point in &expected.checkpoints {
        while frame < point.frame {
            // A multi-frame fixture has one total instruction budget, shared
            // by return detection and scanout advancement, rather than a fresh
            // allowance on every frame or checkpoint.
            let remaining = fixture
                .max_instructions
                .saturating_sub(machine.executed_instructions());
            machine.run_until_pc(expected.return_pc, remaining, Cycle(fixture.max_cycles))?;
            if machine.is_thumb() {
                return Err(fail("counter failed to return in ARM state"));
            }
            frame += 1;
            let remaining = fixture
                .max_instructions
                .saturating_sub(machine.executed_instructions());
            machine.advance_to(Cycle(u64::from(frame) * CYCLES_PER_FRAME), remaining)?;
            if machine.cycles().0 > fixture.max_cycles {
                return Err(fail("counter exceeded cycle limit"));
            }
        }
        let actual = [
            machine.inspect16(fixture.mailbox_address)?,
            machine.inspect16(fixture.mailbox_address + 2)?,
            machine.inspect16(fixture.mailbox_address + 4)?,
        ];
        if actual != [fixture.completion_id, point.count, point.frame]
            || machine.framebuffer_generation() != u64::from(point.frame)
        {
            return Err(fail(format!(
                "counter frame {} unexpected mailbox {actual:?}",
                point.frame
            )));
        }
        for (offset, wanted) in [(8, expected.adr_address), (12, expected.observed_pc)] {
            let actual = u32::from(machine.inspect16(fixture.mailbox_address + offset)?)
                | (u32::from(machine.inspect16(fixture.mailbox_address + offset + 2)?) << 16);
            if actual != wanted {
                return Err(fail(format!(
                    "counter PC observation expected {wanted:#010x}, got {actual:#010x}"
                )));
            }
        }
        for (index, &pixel) in machine.framebuffer().iter().enumerate() {
            let x = index % SCREEN_WIDTH;
            let y = index / SCREEN_WIDTH;
            let wanted = if (72..88).contains(&y)
                && (56..56 + usize::from(point.image_count) * 8).contains(&x)
            {
                0x03e0
            } else {
                0
            };
            if pixel != wanted {
                return Err(fail(format!(
                    "counter frame {} pixel ({x},{y}) expected {wanted:#06x}, got {pixel:#06x}",
                    point.frame
                )));
            }
        }
        if let Some(path) = capture_path {
            capture(
                Path::new(&format!("{path}.counter-frame-{}.ppm", point.frame)),
                machine.framebuffer(),
            )?;
        }
        println!(
            "PASS counter frame={} count={} image_count={} return=ARM pixels={}",
            point.frame,
            point.count,
            point.image_count,
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
    }
    println!(
        "PASS counter cycles={} instructions={}",
        machine.cycles().0,
        machine.executed_instructions()
    );
    Ok(())
}
