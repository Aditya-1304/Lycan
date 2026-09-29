#![forbid(unsafe_code)]

use gba_core::{CYCLES_PER_FRAME, Cycle, Machine, SCREEN_HEIGHT, SCREEN_WIDTH};
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
    terminal_pc: u32,
    mailbox_address: u32,
    completion_id: u16,
    result: u16,
    max_instructions: usize,
    max_cycles: u64,
    band_height: usize,
    colors: Vec<u16>,
    input_events: Vec<String>,
    time_events: Vec<String>,
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
    if manifest.version != 1 || manifest.fixture.is_empty() {
        return Err(fail("unsupported or empty fixture manifest"));
    }
    for fixture in &manifest.fixture {
        if fixture.startup != "cartridge-direct-arm"
            || fixture.bios_required
            || !fixture.input_events.is_empty()
            || !fixture.time_events.is_empty()
            || fixture.band_height == 0
            || fixture.band_height * fixture.colors.len() != SCREEN_HEIGHT
            || fixture.max_instructions == 0
            || fixture.max_cycles == 0
            || fixture.origin.is_empty()
        {
            return Err(fail(format!(
                "invalid Slice 1 fixture configuration: {}",
                fixture.name
            )));
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
fn execute(fixture: &Fixture, bytes: &[u8]) -> Result<Machine> {
    let mut machine = Machine::new();
    machine.load_rom(bytes)?;
    machine.run_until_pc(
        fixture.terminal_pc,
        fixture.max_instructions,
        Cycle(fixture.max_cycles),
    )?;
    let id = machine.inspect16(fixture.mailbox_address)?;
    let result = machine.inspect16(fixture.mailbox_address + 2)?;
    if id != fixture.completion_id || result != fixture.result {
        return Err(fail(format!(
            "{} completion mailbox mismatch: id={id:#06x}, result={result:#06x}",
            fixture.name
        )));
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
    if machine.framebuffer_generation() == 0 {
        return Err(fail("guest produced no completed frame"));
    }
    for (index, &pixel) in machine.framebuffer().iter().enumerate() {
        let expected = fixture.colors[(index / SCREEN_WIDTH) / fixture.band_height];
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

fn capture(path: &Path, machine: &Machine) -> Result<()> {
    let mut output = format!("P6\n{SCREEN_WIDTH} {SCREEN_HEIGHT}\n255\n").into_bytes();
    for &pixel in machine.framebuffer() {
        for shift in [0, 5, 10] {
            let value = ((pixel >> shift) & 31) as u8;
            output.push((value << 3) | (value >> 2));
        }
    }
    fs::write(path, output)?;
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
        let start = Instant::now();
        let mut machine = execute(fixture, &bytes)?;
        let boot_ms = start.elapsed().as_secs_f64() * 1000.0;
        check_image(fixture, &machine)?;
        println!(
            "PASS {} pixels={} terminal={:#010x} cycles={} generation={} execution_ms={boot_ms:.3}",
            fixture.name,
            SCREEN_WIDTH * SCREEN_HEIGHT,
            fixture.terminal_pc,
            machine.cycles().0,
            machine.framebuffer_generation()
        );
        if let Some(path) = option(&args, "--capture")? {
            capture(Path::new(&path), &machine)?;
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
        } else {
            prove_store_effect(fixture, &bytes)?;
        }
    }
    Ok(())
}
