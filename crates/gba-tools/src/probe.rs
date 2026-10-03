//! Supplied-BIOS boot checks and bounded cartridge probes with reproducible evidence.

use crate::bios_contract;
use crate::support::{Result, capture, fail, hash, option};
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine};
use gba_session::Button;
use std::fs;
use std::path::Path;

/// Runs bounded private cartridges and prints reproducible evidence even on
/// failure. A completed probe is an observation, never a gameplay acceptance.
pub(crate) fn probe_bios(args: &[String], diagnostic: bool) -> Result<()> {
    let bios_path = option(args, "--bios")?.ok_or_else(|| fail("--bios PATH is required"))?;
    let bios = fs::read(bios_path)?;
    if diagnostic {
        if hash(bios_contract::ROM)
            != "9d7b369fa1aa661ff03692b3d79c6f644b623d72983d0fc890e6d87a0409a3c9"
        {
            return Err(fail("bios.gba differs from pinned identity"));
        }
        let machine = bios_contract::verify(&bios).map_err(fail)?;
        let pixels: Vec<u8> = machine
            .framebuffer()
            .iter()
            .flat_map(|p| p.to_le_bytes())
            .collect();
        let expected_frame = option(args, "--expected-frame-sha256")?.unwrap_or_else(|| {
            "59ce42abae9825c2d2579c5cd838e47d88be917e37ea36ff162d46fc5d0991e3".to_owned()
        });
        if hash(&pixels) != expected_frame {
            return Err(fail("bios.gba success framebuffer mismatch"));
        }
        if let Some(path) = option(args, "--capture")? {
            capture(Path::new(&path), machine.framebuffer())?;
        }
        println!(
            "PASS bios.gba bios_sha256={} cycles={} instructions={} frames={}",
            hash(&bios),
            machine.cycles().0,
            machine.executed_instructions(),
            machine.framebuffer_generation()
        );
        return Ok(());
    }
    let rom_path = option(args, "--rom")?.ok_or_else(|| fail("--rom PATH is required"))?;
    let rom = fs::read(&rom_path)?;
    let frames: u64 = option(args, "--frames")?
        .unwrap_or_else(|| "600".to_owned())
        .parse()?;
    if !(1..=1800).contains(&frames) {
        return Err(fail("probe frames must be 1..=1800"));
    }
    let backup = option(args, "--backup")?
        .map(|value| value.parse())
        .transpose()?;
    let mut machine = Machine::new();
    machine.load_bios(&bios)?;
    machine.boot_rom_with_bios(&rom, backup)?;
    // Raw backup images are deliberate probe inputs, separate from app envelopes.
    if let Some(path) = option(args, "--load-save")? {
        machine.load_save(&fs::read(path)?).map_err(fail)?;
    }
    #[cfg(not(feature = "eeprom-trace"))]
    if option(args, "--trace-eeprom")?.is_some() {
        return Err(fail("--trace-eeprom requires --features eeprom-trace"));
    }
    #[cfg(feature = "eeprom-trace")]
    let mut trace_file = option(args, "--trace-eeprom")?
        .map(fs::File::create)
        .transpose()?;
    #[cfg(feature = "eeprom-trace")]
    if let Some(file) = &mut trace_file {
        use std::io::Write as _;
        writeln!(
            file,
            "rom_sha256={} bios_sha256={} frames={} press_start={} save_input={:?}",
            hash(&rom),
            hash(&bios),
            frames,
            args.iter().any(|arg| arg == "--press-start"),
            option(args, "--load-save")?
        )?;
        machine.enable_eeprom_trace();
    }
    let mut entered_rom = false;
    let mut failure = None;
    let mut pcm = Vec::new();
    for frame in 1..=frames {
        // Optional startup input is fixed to a documented emulated deadline.
        if args.iter().any(|arg| arg == "--press-start") {
            if frame == 360 {
                machine.set_button(Button::Start, true);
            }
            if frame == 362 {
                machine.set_button(Button::Start, false);
            }
        }
        let target = Cycle(frame * CYCLES_PER_FRAME);
        // Observe the BIOS-to-cartridge handoff at instruction boundaries.
        let initial = machine.executed_instructions();
        while machine.cycles() < target {
            entered_rom |= (0x08000000..0x0e000000).contains(&machine.registers()[15]);
            if machine.executed_instructions().saturating_sub(initial) >= 200_000 {
                failure = Some("per-frame instruction limit exceeded".to_owned());
                break;
            }
            if let Err(error) = machine.step() {
                failure = Some(error.to_string());
                break;
            }
        }
        #[cfg(feature = "eeprom-trace")]
        if let Some(file) = &mut trace_file {
            use std::io::Write as _;
            let (events, dropped) = machine.drain_eeprom_trace();
            for event in events {
                writeln!(file, "cycle={} {:?}", event.cycle, event.event)?;
            }
            if dropped != 0 {
                writeln!(file, "TRACE_INCOMPLETE dropped={dropped}")?;
            }
            // Preserve every completed frame's evidence even if the probe fails.
            file.flush()?;
        }
        machine.drain_stereo_pcm(&mut pcm);
        pcm.clear();
        if failure.is_some() {
            break;
        }
    }
    let pixels: Vec<u8> = machine
        .framebuffer()
        .iter()
        .flat_map(|p| p.to_le_bytes())
        .collect();
    println!(
        "PROBE rom={} size={} sha256={} bios_sha256={}",
        rom_path,
        rom.len(),
        hash(&rom),
        hash(&bios)
    );
    println!(
        "header_title={:?} game_code={:?} revision={:?}",
        rom.get(0xa0..0xac).map(String::from_utf8_lossy),
        rom.get(0xac..0xb0).map(String::from_utf8_lossy),
        rom.get(0xbc)
    );
    println!(
        "entered_rom={} pc={:#010x} cycles={} instructions={} frames={} framebuffer_sha256={} backup={:?} pcm={:?} error={:?}",
        entered_rom,
        machine.registers()[15],
        machine.cycles().0,
        machine.executed_instructions(),
        machine.framebuffer_generation(),
        hash(&pixels),
        machine.backup_selection(),
        machine.pcm_counters(),
        failure
    );
    if let Some(path) = option(args, "--capture")? {
        capture(Path::new(&path), machine.framebuffer())?;
    }
    let save = machine.save_image();
    if let Some(path) = option(args, "--save-out")? {
        let image = save
            .as_ref()
            .ok_or_else(|| fail("cartridge has no backup image"))?;
        if image.bytes.is_empty() {
            return Err(fail("backup capacity is unresolved; no image exported"));
        }
        fs::write(path, &image.bytes)?;
    }
    machine.reset();
    let reset_save = machine.save_image();
    if machine.registers()[15] != 0
        || save.as_ref().map(|s| (&s.bytes, s.revision))
            != reset_save.as_ref().map(|s| (&s.bytes, s.revision))
    {
        return Err(fail("BIOS reset/save preservation failed"));
    }
    println!("reset_pc=0x00000000 backup_preserved=true");
    if let Some(error) = failure {
        return Err(fail(error));
    }
    if !entered_rom {
        return Err(fail(
            "BIOS did not transfer control to cartridge within probe bounds",
        ));
    }
    Ok(())
}
