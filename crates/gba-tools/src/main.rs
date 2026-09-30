#![forbid(unsafe_code)]

use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_HEIGHT, SCREEN_WIDTH};
use gba_session::{Button, Session};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// App replay input is checked against the independent frozen manifest below.
const DEMO_INPUT: &[(Cycle, Button, bool)] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/buttons/input.rs"
));

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    fixture: Vec<Fixture>,
}

/// The frozen manifest specifies completion independently of the guest implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    name: String,
    rom: String,
    source: String,
    linker: String,
    sha256: String,
    origin: String,
    startup: String,
    bios_required: bool,
    mailbox_address: u32,
    completion_id: u16,
    max_instructions: usize,
    max_cycles: u64,
    verification: Verification,
}

/// Each fixture declares its own observable completion rather than relying on
/// an arbitrary guest loop. Moving scenes require bounded checkpoints.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Verification {
    Pixels(Pixels),
    Calculations(Pixels),
    Buttons(Movement),
    Palette(Palette),
    Hello(Hello),
}

/// Pinned upstream hello completion and an independently derived full-frame
/// digest. The reference ROM has no original-fixture completion mailbox.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hello {
    terminal_pc: u32,
    framebuffer_sha256: String,
}

/// One guest reproduction covers indexed lookup, both pages, and a palette-only
/// change. Frozen colors are independent of the renderer and guest stores.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Palette {
    input_events: Vec<InputEvent>,
    checkpoints: Vec<PaletteCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaletteCheckpoint {
    frame: u16,
    page: u16,
    palette_changed: u16,
    colors: [u16; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pixels {
    terminal_pc: u32,
    result: u16,
    band_height: usize,
    colors: Vec<u16>,
    input_events: Vec<String>,
    time_events: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Movement {
    square_size: usize,
    color: u16,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<Checkpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputEvent {
    cycle: u64,
    button: String,
    pressed: bool,
}

/// RAM reports current guest state; scanout contains the preceding frame's
/// drawing because the guest changes VRAM after publication at VBlank.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    frame: u16,
    x: u16,
    y: u16,
    image_x: usize,
    image_y: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

fn fail(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::other(message.into()).into()
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest(path: &Path) -> Result<Manifest> {
    let manifest: Manifest = toml::from_str(&fs::read_to_string(path)?)?;
    if manifest.version != 2 || manifest.fixture.is_empty() {
        return Err(fail("unsupported or empty fixture manifest"));
    }
    for fixture in &manifest.fixture {
        if fixture.startup != "cartridge-direct-arm"
            || fixture.bios_required
            || fixture.max_instructions == 0
            || fixture.max_cycles == 0
            || fixture.origin.is_empty()
        {
            return Err(fail(format!(
                "invalid fixture configuration: {}",
                fixture.name
            )));
        }
        match &fixture.verification {
            Verification::Hello(expected) => {
                if expected.terminal_pc & 3 != 0 || expected.framebuffer_sha256.len() != 64 {
                    return Err(fail("invalid hello expectation"));
                }
            }
            Verification::Palette(expected) => {
                if expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|pair| pair[0].frame >= pair[1].frame)
                    || expected.checkpoints.iter().any(|point| {
                        point.frame == 0
                            || point.page > 1
                            || point.palette_changed > 1
                            || u64::from(point.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                            || point.colors.iter().any(|color| *color > 0x7fff)
                    })
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > fixture.max_cycles)
                {
                    return Err(fail("invalid palette checkpoint"));
                }
            }
            Verification::Pixels(expected) | Verification::Calculations(expected) => {
                if !expected.input_events.is_empty()
                    || !expected.time_events.is_empty()
                    || expected.band_height == 0
                    || expected.band_height.checked_mul(expected.colors.len())
                        != Some(SCREEN_HEIGHT)
                {
                    return Err(fail("invalid pixels expectation"));
                }
            }
            Verification::Buttons(expected) => {
                if expected.square_size == 0
                    || expected.square_size > SCREEN_HEIGHT
                    || expected.color == 0
                    || expected.color > 0x7fff
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|pair| pair[0].frame >= pair[1].frame)
                    || expected
                        .input_events
                        .windows(2)
                        .any(|pair| pair[0].cycle > pair[1].cycle)
                {
                    return Err(fail("invalid movement script"));
                }
                for checkpoint in &expected.checkpoints {
                    if checkpoint.frame == 0
                        || u64::from(checkpoint.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                        || checkpoint.image_x + expected.square_size > SCREEN_WIDTH
                        || checkpoint.image_y + expected.square_size > SCREEN_HEIGHT
                        || usize::from(checkpoint.x) + expected.square_size > SCREEN_WIDTH
                        || usize::from(checkpoint.y) + expected.square_size > SCREEN_HEIGHT
                    {
                        return Err(fail("movement checkpoint outside fixture bounds"));
                    }
                }
                for event in &expected.input_events {
                    button(&event.button)?;
                    if event.cycle > fixture.max_cycles {
                        return Err(fail("input event exceeds fixture cycle limit"));
                    }
                }
            }
        }
    }
    Ok(manifest)
}

fn tool(program: &str, args: &[&std::ffi::OsStr]) -> Result<()> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(fail(format!("{program} failed with {status}")));
    }
    Ok(())
}

/// Rebuilds original assembly with GNU Arm binutils and verifies its frozen identity.
fn build_fixtures(path: &Path) -> Result<()> {
    let manifest = manifest(path)?;
    let directory = path.parent().unwrap();
    let build = std::env::temp_dir().join(format!("gba-fixtures-{}", std::process::id()));
    fs::create_dir_all(&build)?;
    for fixture in &manifest.fixture {
        if matches!(fixture.verification, Verification::Hello(_)) {
            // Upstream distributes a FASMARM binary. Verify its identity rather
            // than attempting to assemble its source with GNU Arm binutils.
            load(fixture, directory)?;
            println!(
                "VERIFIED {} upstream binary sha256={}",
                fixture.name, fixture.sha256
            );
            continue;
        }
        let source = directory.join(&fixture.source);
        let linker = directory.join(&fixture.linker);
        let object = build.join(format!("{}.o", fixture.name));
        let elf = build.join(format!("{}.elf", fixture.name));
        let binary = build.join(format!("{}.gba", fixture.name));
        tool(
            "arm-none-eabi-as",
            &[
                "-mcpu=arm7tdmi".as_ref(),
                "-o".as_ref(),
                object.as_os_str(),
                source.as_os_str(),
            ],
        )?;
        tool(
            "arm-none-eabi-ld",
            &[
                "-T".as_ref(),
                linker.as_os_str(),
                "-o".as_ref(),
                elf.as_os_str(),
                object.as_os_str(),
            ],
        )?;
        tool(
            "arm-none-eabi-objcopy",
            &[
                "-O".as_ref(),
                "binary".as_ref(),
                elf.as_os_str(),
                binary.as_os_str(),
            ],
        )?;
        let bytes = fs::read(binary)?;
        if hash(&bytes) != fixture.sha256 {
            return Err(fail(format!(
                "{} rebuild differs from frozen SHA-256; review the source and manifest before accepting a new fixture",
                fixture.name
            )));
        }
        fs::write(directory.join(&fixture.rom), &bytes)?;
        println!(
            "BUILT {} bytes={} sha256={}",
            fixture.name,
            bytes.len(),
            fixture.sha256
        );
    }
    Ok(())
}

fn load(fixture: &Fixture, directory: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(directory.join(&fixture.rom))?;
    let identity = hash(&bytes);
    if identity != fixture.sha256 {
        return Err(fail(format!(
            "{} SHA-256 mismatch: expected {}, got {identity}",
            fixture.name, fixture.sha256
        )));
    }
    Ok(bytes)
}

/// Requires the exact executed terminal PC and both diagnostic RAM values.
fn pixels(fixture: &Fixture) -> Result<&Pixels> {
    match &fixture.verification {
        Verification::Pixels(expected) | Verification::Calculations(expected) => Ok(expected),
        Verification::Buttons(_) | Verification::Palette(_) | Verification::Hello(_) => {
            Err(fail("expected a terminal pixels fixture"))
        }
    }
}

fn execute(fixture: &Fixture, bytes: &[u8]) -> Result<Machine> {
    let expected = pixels(fixture)?;
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.run_until_pc(
        expected.terminal_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
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

fn check_image(fixture: &Fixture, machine: &Machine) -> Result<()> {
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
fn prove_store_effect(fixture: &Fixture, bytes: &[u8]) -> Result<()> {
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

fn capture(path: &Path, framebuffer: &[u16]) -> Result<()> {
    let mut output = format!("P6\n{SCREEN_WIDTH} {SCREEN_HEIGHT}\n255\n").into_bytes();
    for &pixel in framebuffer {
        for shift in [0, 5, 10] {
            let value = ((pixel >> shift) & 31) as u8;
            output.push((value << 3) | (value >> 2));
        }
    }
    fs::write(path, output)?;
    Ok(())
}

/// Resolves fixture input names to the same logical buttons used by the app.
fn button(name: &str) -> Result<Button> {
    match name {
        "A" => Ok(Button::A),
        "B" => Ok(Button::B),
        "Select" => Ok(Button::Select),
        "Start" => Ok(Button::Start),
        "Right" => Ok(Button::Right),
        "Left" => Ok(Button::Left),
        "Up" => Ok(Button::Up),
        "Down" => Ok(Button::Down),
        "R" => Ok(Button::R),
        "L" => Ok(Button::L),
        _ => Err(fail(format!("unknown logical button: {name}"))),
    }
}

/// Executes the original square guest through the shared session. Every checkpoint
/// verifies a declared mailbox and all pixels, including erased square positions.
fn run_movement(
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
fn run_palette(
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

fn option(args: &[String], key: &str) -> Result<Option<String>> {
    match args.iter().position(|arg| arg == key) {
        Some(index) => args
            .get(index + 1)
            .cloned()
            .map(Some)
            .ok_or_else(|| fail(format!("missing value for {key}"))),
        None => Ok(None),
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = option(&args, "--manifest")?
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("fixtures/manifest.toml"));
    if args.first().map(String::as_str) == Some("build-fixtures") {
        return build_fixtures(&path);
    }
    let bench = args.first().map(String::as_str) == Some("bench");
    if !args.is_empty()
        && !bench
        && args[0] != "pixels"
        && !(args[0] == "fixtures" && args.get(1).map(String::as_str) == Some("run"))
    {
        return Err(fail(
            "usage: build-fixtures | fixtures run --manifest PATH [--capture PATH] | bench --scenario pixels --frames N",
        ));
    }
    if bench && option(&args, "--scenario")?.as_deref().unwrap_or("pixels") != "pixels" {
        return Err(fail("only the Slice 1 pixels benchmark is available"));
    }
    let manifest = manifest(&path)?;
    for fixture in &manifest.fixture {
        let bytes = load(fixture, path.parent().unwrap())?;
        if let Verification::Buttons(expected) = &fixture.verification {
            if bench {
                continue;
            }
            run_movement(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            continue;
        }
        if let Verification::Palette(expected) = &fixture.verification {
            if !bench {
                run_palette(
                    fixture,
                    expected,
                    &bytes,
                    option(&args, "--capture")?.as_deref(),
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
                if let Some(path) = option(&args, "--capture")? {
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
        if let Some(path) = option(&args, "--capture")? {
            capture(Path::new(&path), machine.framebuffer())?;
        }
        if bench {
            let frames: usize = option(&args, "--frames")?
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
