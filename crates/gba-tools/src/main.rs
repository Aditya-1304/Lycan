#![forbid(unsafe_code)]

#[path = "../../../roms/blend/contract.rs"]
mod blend_contract;
#[path = "../../../roms/affine-object/contract.rs"]
mod object_contract;
#[path = "../../../roms/raster/contract.rs"]
mod raster_contract;
#[path = "../../../roms/window/contract.rs"]
mod window_contract;

#[path = "../../../roms/affine/contract.rs"]
mod affine_contract;

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

#[path = "../../../roms/bios/contract.rs"]
mod bios_contract;
#[path = "../../../roms/noise/contract.rs"]
mod noise_contract;
#[path = "../../../roms/pulse/contract.rs"]
mod pulse_contract;
#[path = "../../../roms/wave/contract.rs"]
mod wave_contract;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// App replay input is checked against the independent frozen manifest below.
const DEMO_INPUT: &[(Cycle, Button, bool)] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/buttons/input.rs"
));

/// The app replay is checked against the separately frozen fixture timeline.
const TILED_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/tiled/input.rs");

/// The application timeline must match independently retained fixture events.
const SPRITE_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/sprites/input.rs");
const DMA_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/dma/input.rs");

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
    Copy(Pixels),
    Buttons(Movement),
    Palette(Palette),
    Hello(Hello),
    Counter(Counter),
    Tiled(Tiled),
    Sprites(Sprites),
    Vblank(Vblank),
    Keypad(Keypad),
    Dma(DmaScene),
    Pcm(PcmScene),
    Stripes(Stripes),
    Timing(Timing),
    Diagnostic(Diagnostic),
}

/// Independently frozen guest startup, input and audiovisual checkpoints.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PcmScene {
    ready_pc: u32,
    firmware_sha256: String,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<PcmCheckpoint>,
    mixer_input_events: Vec<InputEvent>,
    mixer_checkpoints: Vec<MixerCheckpoint>,
    mixer_fifo_underruns: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PcmCheckpoint {
    frame: u16,
    color: u16,
    pressed: u16,
    transitions: u16,
    samples: usize,
    pcm_sha256: String,
}

/// Stereo DAC levels are expressed in signed 10-bit units, before host gain.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MixerCheckpoint {
    frame: u16,
    control_h: u16,
    master: u16,
    bias: u16,
    levels: Vec<[i16; 2]>,
    pcm_sha256: String,
}

const MIXER_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/pcm/mixer_input.rs");

const PCM_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/pcm/input.rs");

/// The DMA scene has an independently frozen startup PC and firmware identity.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DmaScene {
    ready_pc: u32,
    firmware_sha256: String,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<DmaCheckpoint>,
}

/// Frozen data, complete scanout and interrupt counts at one frame boundary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DmaCheckpoint {
    frame: u16,
    tile: u16,
    color: u16,
    vblank: u16,
    completions: u16,
}

/// Frozen keypad mode and mapped firmware identity for bounded guest acceptance.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Keypad {
    and_mode: bool,
    firmware_sha256: String,
}

const KEYPAD_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/keypad/input.rs");

/// IRQ guest identity and bounded completion; configuration variants only change
/// the declared source/master/CPU mask word in the verified original ROM.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vblank {
    configuration_offset: usize,
    frames: u16,
    firmware_sha256: String,
}

/// Scripted guest offsets and independently computed complete tile-scene captures.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tiled {
    ready_pc: u32,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<TiledCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TiledCheckpoint {
    frame: u16,
    updates: u16,
    x: u16,
    y: u16,
    image_x: usize,
    image_y: usize,
}

/// Frozen player/OAM state accompanies the independent background checkpoints.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sprites {
    /// Present only for the IRQ scene; the polling fixture leaves firmware unmapped.
    #[serde(default)]
    firmware_sha256: Option<String>,
    ready_pc: u32,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<SpriteCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteCheckpoint {
    background: TiledCheckpoint,
    x: u16,
    y: u16,
    image_x: usize,
    image_y: usize,
    priority: u16,
    image_priority: u16,
    mapping_1d: bool,
    image_mapping_1d: bool,
}

/// Upstream success is tied to its source-defined idle PC and exact stripe colors.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stripes {
    terminal_pc: u32,
}

/// Frozen diagnostic success requires a specific opcode, status, result, and
/// original firmware identity. An unrelated self-branch never satisfies it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    /// Pinned homebrew without an SDK version uses the centralized typed override.
    backup_override: Option<String>,
    terminal_pc: u32,
    terminal_instruction: u32,
    cpsr_mask: u32,
    cpsr_value: u32,
    result_register: usize,
    result: u32,
    firmware_sha256: String,
    framebuffer_sha256: Option<String>,
}

/// Independently derived timing intervals are measured after both marker
/// instructions, including CPU pipeline fetches but excluding setup/reporting.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Timing {
    terminal_pc: u32,
    checkpoints: Vec<TimingCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimingCheckpoint {
    start_pc: u32,
    end_pc: u32,
    cycles: u64,
    waitcnt: u16,
    width: u32,
    window: u32,
}

/// Frozen observations bind input, ARM return, Thumb PC semantics, and scanout.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Counter {
    return_pc: u32,
    adr_address: u32,
    observed_pc: u32,
    input_events: Vec<InputEvent>,
    checkpoints: Vec<CounterCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CounterCheckpoint {
    frame: u16,
    count: u16,
    image_count: u16,
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
    /// Frozen halfwords checked independently of the displayed frame.
    #[serde(default)]
    memory_checks: Vec<MemoryCheck>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryCheck {
    address: u32,
    value: u16,
    /// Number of consecutive halfwords; singleton probes omit this field.
    #[serde(default = "one_halfword")]
    halfwords: u32,
}

fn one_halfword() -> u32 {
    1
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
            || (fixture.bios_required
                != (matches!(
                    fixture.verification,
                    Verification::Diagnostic(_)
                        | Verification::Vblank(_)
                        | Verification::Keypad(_)
                        | Verification::Dma(_)
                        | Verification::Pcm(_)
                ) || matches!(&fixture.verification, Verification::Sprites(expected) if expected.firmware_sha256.is_some())))
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
            Verification::Pcm(expected) => {
                let events = expected
                    .input_events
                    .iter()
                    .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
                    .collect::<Result<Vec<_>>>()?;
                let mixer_events = expected
                    .mixer_input_events
                    .iter()
                    .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
                    .collect::<Result<Vec<_>>>()?;
                if mixer_events != MIXER_INPUT
                    || expected.mixer_checkpoints.is_empty()
                    || expected
                        .mixer_checkpoints
                        .windows(2)
                        .any(|p| p[0].frame >= p[1].frame)
                    || expected.mixer_checkpoints.iter().any(|p| {
                        p.levels.is_empty()
                            || p.pcm_sha256.len() != 64
                            || u64::from(p.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                    })
                    || events != PCM_INPUT
                    || expected.ready_pc & 3 != 0
                    || expected.firmware_sha256 != hash(gba_core::TEST_FIRMWARE)
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|p| p[0].frame >= p[1].frame)
                    || expected.checkpoints.iter().any(|p| {
                        p.frame == 0
                            || u64::from(p.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                            || p.pcm_sha256.len() != 64
                    })
                {
                    return Err(fail("invalid PCM identity, timeline or bounds"));
                }
            }
            Verification::Dma(expected) => {
                if expected.ready_pc & 3 != 0
                    || expected.firmware_sha256.len() != 64
                    || fixture.max_cycles < 10 * CYCLES_PER_FRAME
                {
                    return Err(fail("invalid DMA scene bounds or identity"));
                }
                let events = expected
                    .input_events
                    .iter()
                    .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
                    .collect::<Result<Vec<_>>>()?;
                if events != DMA_INPUT
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|points| points[0].frame >= points[1].frame)
                    || expected.checkpoints.iter().any(|point| {
                        point.frame == 0
                            || u64::from(point.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                    })
                {
                    return Err(fail(
                        "invalid DMA replay or app timeline differs from manifest",
                    ));
                }
            }
            Verification::Keypad(expected) => {
                if expected.firmware_sha256.len() != 64 || fixture.max_cycles < 2 * CYCLES_PER_FRAME
                {
                    return Err(fail("invalid keypad identity or cycle bound"));
                }
            }
            Verification::Vblank(expected) => {
                if expected.frames < 2
                    || u64::from(expected.frames) * CYCLES_PER_FRAME > fixture.max_cycles
                    || expected.configuration_offset != 0x300
                    || expected.firmware_sha256.len() != 64
                {
                    return Err(fail("invalid VBlank fixture bounds or identity"));
                }
            }

            Verification::Sprites(expected) => {
                if expected
                    .firmware_sha256
                    .as_ref()
                    .is_some_and(|hash| hash.len() != 64)
                {
                    return Err(fail("invalid IRQ scene firmware identity"));
                }
                if expected.ready_pc & 3 != 0
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|p| p[0].background.frame >= p[1].background.frame)
                    || expected.checkpoints.iter().any(|p| {
                        let b = &p.background;
                        b.frame == 0
                            || u64::from(b.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                            || b.x > 511
                            || b.y > 511
                            || b.image_x > 511
                            || b.image_y > 511
                            || p.x > 224
                            || p.y > 144
                            || p.image_x > 224
                            || p.image_y > 144
                            || p.priority > 1
                            || p.image_priority > 1
                    })
                    || expected
                        .input_events
                        .windows(2)
                        .any(|p| p[0].cycle > p[1].cycle)
                    || expected
                        .input_events
                        .iter()
                        .any(|p| p.cycle > fixture.max_cycles)
                {
                    return Err(fail("invalid sprite replay"));
                }
                let events: Vec<_> = expected
                    .input_events
                    .iter()
                    .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
                    .collect::<Result<_>>()?;
                if events != SPRITE_INPUT {
                    return Err(fail("app sprite replay differs from frozen manifest"));
                }
            }
            Verification::Tiled(expected) => {
                if expected.ready_pc & 3 != 0
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|p| p[0].frame >= p[1].frame)
                    || expected.checkpoints.iter().any(|p| {
                        p.frame == 0
                            || u64::from(p.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                            || p.x > 511
                            || p.y > 511
                            || p.image_x > 511
                            || p.image_y > 511
                    })
                {
                    return Err(fail("invalid tiled checkpoints"));
                }
                if expected
                    .input_events
                    .windows(2)
                    .any(|p| p[0].cycle > p[1].cycle)
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > fixture.max_cycles)
                {
                    return Err(fail("invalid tiled input timeline"));
                }
                let events: Vec<_> = expected
                    .input_events
                    .iter()
                    .map(|event| Ok((Cycle(event.cycle), button(&event.button)?, event.pressed)))
                    .collect::<Result<_>>()?;
                if events != TILED_INPUT {
                    return Err(fail("app tiled replay differs from frozen manifest"));
                }
            }
            Verification::Stripes(expected) => {
                if expected.terminal_pc & 3 != 0 {
                    return Err(fail("unaligned stripes idle PC"));
                }
            }
            Verification::Diagnostic(expected) => {
                if expected.terminal_pc & 3 != 0
                    || expected.terminal_instruction != 0xeafffffe
                    || expected.result_register >= 15
                    || expected.cpsr_mask & 0x3f != 0x3f
                    || expected.cpsr_value & 0x20 != 0
                    || expected.cpsr_value & !expected.cpsr_mask != 0
                    || expected.firmware_sha256.len() != 64
                    || expected
                        .framebuffer_sha256
                        .as_ref()
                        .is_some_and(|hash| hash.len() != 64)
                {
                    return Err(fail("invalid diagnostic completion contract"));
                }
            }
            Verification::Timing(expected) => {
                if expected.checkpoints.len() != 24
                    || expected.terminal_pc & 3 != 0
                    || expected.checkpoints.iter().any(|point| {
                        if point.window > 2 {
                            return true;
                        }
                        let base = 0x08000000 + point.window * 0x02000000;
                        !matches!(point.width, 16 | 32)
                            || point.start_pc < base
                            || point.end_pc >= base + 0x02000000
                            || point.start_pc >= point.end_pc
                            || point.cycles == 0
                            || point.cycles > fixture.max_cycles
                            || point.waitcnt & !0x5fff != 0
                    })
                {
                    return Err(fail("invalid cartridge timing checkpoints"));
                }
            }
            Verification::Counter(expected) => {
                if expected.return_pc & 3 != 0
                    || expected.adr_address & 3 != 0
                    || expected.checkpoints.is_empty()
                    || expected
                        .checkpoints
                        .windows(2)
                        .any(|pair| pair[0].frame >= pair[1].frame)
                    || expected.checkpoints.iter().any(|point| {
                        point.frame == 0
                            || point.count > 16
                            || point.image_count > 16
                            || u64::from(point.frame) * CYCLES_PER_FRAME > fixture.max_cycles
                    })
                    || expected
                        .input_events
                        .windows(2)
                        .any(|pair| pair[0].cycle > pair[1].cycle)
                {
                    return Err(fail("invalid counter expectations"));
                }
                for event in &expected.input_events {
                    button(&event.button)?;
                    if event.cycle > fixture.max_cycles {
                        return Err(fail("counter input exceeds cycle limit"));
                    }
                }
            }
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
            Verification::Pixels(expected)
            | Verification::Calculations(expected)
            | Verification::Copy(expected) => {
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
    if manifest.fixture.iter().any(|fixture| fixture.bios_required) {
        let object = build.join("division.o");
        let elf = build.join("division.elf");
        let binary = build.join("division.bin");
        tool(
            "arm-none-eabi-as",
            &[
                "-mcpu=arm7tdmi".as_ref(),
                "-o".as_ref(),
                object.as_os_str(),
                root().join("roms/test-firmware/division.s").as_os_str(),
            ],
        )?;
        tool(
            "arm-none-eabi-ld",
            &[
                "-T".as_ref(),
                root().join("roms/test-firmware/linker.ld").as_os_str(),
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
        let identity = hash(&bytes);
        for fixture in &manifest.fixture {
            let expected = match &fixture.verification {
                Verification::Diagnostic(expected) => Some(&expected.firmware_sha256),
                Verification::Vblank(expected) => Some(&expected.firmware_sha256),
                Verification::Keypad(expected) => Some(&expected.firmware_sha256),
                Verification::Dma(expected) => Some(&expected.firmware_sha256),
                Verification::Pcm(expected) => Some(&expected.firmware_sha256),
                Verification::Sprites(expected) => expected.firmware_sha256.as_ref(),
                _ => None,
            };
            if expected.is_some_and(|expected| *expected != identity) {
                return Err(fail("rebuilt test firmware differs from frozen SHA-256"));
            }
        }
        fs::write(root().join("roms/test-firmware/division.bin"), bytes)?;
        println!("BUILT original test firmware (SWI 0x06 and IRQ vector)");
    }
    for fixture in &manifest.fixture {
        if matches!(
            fixture.verification,
            Verification::Hello(_) | Verification::Stripes(_)
        ) || (matches!(fixture.verification, Verification::Diagnostic(_))
            && fixture.source.ends_with(".asm"))
        {
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
                "-I".as_ref(),
                root().as_os_str(),
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

fn execute(fixture: &Fixture, bytes: &[u8]) -> Result<Machine> {
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

/// Samples the independent procedural scene after bounded guest execution.
/// The oracle uses world coordinates, not emulated VRAM or renderer helpers.
fn tiled_pixel(index: usize, scroll_x: usize, scroll_y: usize) -> u16 {
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

fn check_tiled_image(machine: &Machine, scroll_x: usize, scroll_y: usize) -> Result<()> {
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
fn check_sprite_image(machine: &Machine, point: &SpriteCheckpoint) -> Result<()> {
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

/// Verifies bus ownership during setup and the two-frame input/upload/scanout
/// pipeline. Full-frame colors and all copied tile words are independent oracles.
/// Executes the same guest used by the app. Every frame drains core PCM so the
/// hash covers real FIFO output without depending on the staging capacity.
fn run_pcm(
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
fn run_mixer(
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

fn run_dma(
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
fn run_keypad(fixture: &Fixture, expected: &Keypad, bytes: &[u8]) -> Result<()> {
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
fn run_vblank(
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

/// One bounded replay checks guest RAM, OAM, scroll registers and full scanout.
fn run_sprites(
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
fn run_tiled(
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
fn run_stripes(
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
fn run_counter(
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

/// Requires exact elapsed cycles, bounded execution, data identity, and the
/// guest's declared mailbox. Reaching a self-branch alone never passes.
fn verify_timing(fixture: &Fixture, bytes: &[u8], expected: &Timing) -> Result<()> {
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
fn run_diagnostic(
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

/// Measures complete frame advances after scene initialization and replay.
/// Host samples are owned by this CLI; the emulation hot path allocates nothing.
fn benchmark_frames(fixture: &Fixture, machine: &mut Machine, frames: usize) -> Result<()> {
    benchmark_machine(&fixture.name, machine, frames, fixture.max_instructions)
}

/// Shares frame timing between manifest fixtures and bounded interactive diagnostics.
fn benchmark_machine(
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

/// Runs bounded private cartridges and prints reproducible evidence even on
/// failure. A completed probe is an observation, never a gameplay acceptance.
fn probe_bios(args: &[String], diagnostic: bool) -> Result<()> {
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

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("verify-raster") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("roms/raster/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("roms/raster.gba"))?;
        if identity["bytes"].as_integer() != Some(bytes.len() as i64) {
            return Err("Raster ROM size mismatch".into());
        }
        let actual_hash = hash(&bytes);
        if identity["sha256"].as_str() != Some(actual_hash.as_str()) {
            return Err("Raster ROM identity mismatch".into());
        }
        let directory = root().join("target/raster-captures");
        fs::create_dir_all(&directory)?;
        raster_contract::verify_with_capture(|frame, pixels| {
            let mut ppm = b"P6\n240 160\n255\n".to_vec();
            for pixel in pixels {
                for shift in [0, 5, 10] {
                    let channel = ((pixel >> shift) & 31) as u8;
                    ppm.push((channel << 3) | (channel >> 2));
                }
            }
            fs::write(directory.join(format!("frame-{frame}.ppm")), ppm)
                .expect("write raster capture");
        });
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-raster") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../roms/raster.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("raster", &mut machine, 600, 2_000_000)?;
        assert_eq!(machine.framebuffer(), image);
        assert_eq!(machine.inspect16(0x0300_0000)?, 0x00a6);
        assert_eq!(machine.inspect16(0x0300_0002)?, 0);
        assert_eq!(machine.inspect16(0x0300_0004)?, 0x0100);
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-blend") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("roms/blend/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("roms/blend.gba"))?;
        if identity["bytes"].as_integer() != Some(bytes.len() as i64) {
            return Err("Blend ROM size mismatch".into());
        }
        let actual_hash = hash(&bytes);
        if identity["sha256"].as_str() != Some(actual_hash.as_str()) {
            return Err("Blend ROM identity mismatch".into());
        }
        let directory = root().join("target/blend-captures");
        fs::create_dir_all(&directory)?;
        blend_contract::verify_with_capture(|frame, pixels| {
            let mut ppm = b"P6\n240 160\n255\n".to_vec();
            for pixel in pixels {
                for shift in [0, 5, 10] {
                    let channel = ((pixel >> shift) & 31) as u8;
                    ppm.push((channel << 3) | (channel >> 2));
                }
            }
            fs::write(directory.join(format!("frame-{frame}.ppm")), ppm).unwrap();
        });
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-blend") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../roms/blend.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("blend", &mut machine, 600, 2_000_000)?;
        assert_eq!(machine.framebuffer(), image);
        assert_eq!(machine.inspect16(0x0300_0000)?, 0x00a5);
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-window") {
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("roms/window/manifest.toml"),
        )?)?;
        let bytes = fs::read(root().join("roms/window.gba"))?;
        if identity["sha256"].as_str() != Some(hash(&bytes).as_str()) {
            return Err("Window ROM identity mismatch".into());
        }
        let directory = root().join("target/window-captures");
        fs::create_dir_all(&directory)?;
        window_contract::verify_with_capture(|frame, pixels| {
            let mut ppm = b"P6\n240 160\n255\n".to_vec();
            for pixel in pixels {
                for shift in [0, 5, 10] {
                    let channel = ((pixel >> shift) & 31) as u8;
                    ppm.push((channel << 3) | (channel >> 2));
                }
            }
            fs::write(directory.join(format!("frame-{frame}.ppm")), ppm).unwrap();
        });
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-window") {
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../roms/window.gba"))?;
        machine.advance_to(Cycle(12 * CYCLES_PER_FRAME), 2_000_000)?;
        let image = machine.framebuffer().to_vec();
        benchmark_machine("window", &mut machine, 600, 2_000_000)?;
        assert_eq!(machine.framebuffer(), image);
        assert_eq!(machine.inspect16(0x03000000)?, 0xa4);
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bench-affine-object") {
        let frames = option(&args, "--frames")?
            .unwrap_or_else(|| "600".to_owned())
            .parse()?;
        let mut machine = Machine::new();
        machine.load_rom(include_bytes!("../../../roms/affine-object.gba"))?;
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
            root().join("roms/affine-object/manifest.toml"),
        )?)?;
        let degenerate = fs::read(root().join("roms/affine-object-degenerate.gba"))?;
        if identity["mgba_degenerate"]["sha256"].as_str() != Some(hash(&degenerate).as_str()) {
            return Err("mGBA degenerate ROM identity mismatch".into());
        }
        let bytes = fs::read(root().join("roms/affine-object.gba"))?;
        if identity["sha256"].as_str() != Some(hash(&bytes).as_str()) {
            return Err("Affine object ROM identity mismatch".into());
        }
        if let Some(directory) = &capture {
            fs::create_dir_all(directory)?;
        }
        let mut capture_image = |frame, pixels: &[u16]| {
            if let Some(directory) = &capture {
                let mut ppm = b"P6\n240 160\n255\n".to_vec();
                for pixel in pixels {
                    for shift in [0, 5, 10] {
                        let channel = ((pixel >> shift) & 31) as u8;
                        ppm.push((channel << 3) | (channel >> 2));
                    }
                }
                fs::write(directory.join(format!("frame-{frame}.ppm")), ppm)
                    .expect("write object capture");
            }
        };
        object_contract::verify_with_capture(&mut capture_image);
        object_contract::verify_degenerate(capture_image);
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("verify-affine") {
        let capture = option(&args, "--capture")?.map(PathBuf::from);
        if let Some(directory) = &capture {
            fs::create_dir_all(directory)?;
        }
        let identity: toml::Value = toml::from_str(&fs::read_to_string(
            root().join("roms/affine/manifest.toml"),
        )?)?;
        for mode in 1..=5 {
            let bytes = fs::read(root().join(format!("roms/affine-mode-{mode}.gba")))?;
            if identity[format!("mode_{mode}")]["sha256"].as_str() != Some(hash(&bytes).as_str()) {
                return Err(format!("Affine mode {mode} ROM identity mismatch").into());
            }
        }
        affine_contract::verify_with_capture(|mode, frame, pixels| {
            if let Some(directory) = &capture {
                let mut ppm = b"P6\n240 160\n255\n".to_vec();
                for pixel in pixels {
                    for shift in [0, 5, 10] {
                        let channel = ((pixel >> shift) & 31) as u8;
                        ppm.push((channel << 3) | (channel >> 2));
                    }
                }
                fs::write(
                    directory.join(format!("mode-{mode}-frame-{frame}.ppm")),
                    ppm,
                )
                .expect("write affine capture");
            }
        });
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
                "roms/noise/manifest.toml"
            } else if wave {
                "roms/wave/manifest.toml"
            } else {
                "roms/pulse/manifest.toml"
            }))?)?;
        let identity = format!(
            "{:x}",
            Sha256::digest(if noise {
                include_bytes!("../../../roms/noise.gba").as_slice()
            } else if wave {
                include_bytes!("../../../roms/wave.gba").as_slice()
            } else {
                include_bytes!("../../../roms/pulse.gba").as_slice()
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
    let bench = args.first().map(String::as_str) == Some("bench");
    if !args.is_empty()
        && !bench
        && args[0] != "pixels"
        && !(args[0] == "fixtures" && args.get(1).map(String::as_str) == Some("run"))
    {
        return Err(fail(
            "usage: verify-noise [--capture WAV] | verify-wave [--capture WAV] | verify-pulse [--capture WAV] | build-fixtures | fixtures run --manifest PATH [--fixture NAME] [--capture PATH] | bench --scenario pixels|tiled|sprites|vblank|irq-sprites|dma|pcm --frames N",
        ));
    }
    let scenario = option(&args, "--scenario")?.unwrap_or_else(|| "pixels".to_owned());
    if bench
        && !matches!(
            scenario.as_str(),
            "pixels" | "tiled" | "sprites" | "vblank" | "irq-sprites" | "dma" | "pcm"
        )
    {
        return Err(fail(
            "benchmark scenario must be pixels, tiled, sprites, vblank, irq-sprites, dma or pcm",
        ));
    }
    let manifest = manifest(&path)?;
    let selected = option(&args, "--fixture")?;
    if selected
        .as_ref()
        .is_some_and(|name| !manifest.fixture.iter().any(|f| &f.name == name))
    {
        return Err(fail("unknown fixture name"));
    }
    if bench && selected.as_ref().is_some_and(|name| name != &scenario) {
        return Err(fail("benchmark fixture must match its scenario"));
    }
    for fixture in &manifest.fixture {
        if selected.as_ref().is_some_and(|name| &fixture.name != name) {
            continue;
        }
        if bench && fixture.name != scenario {
            continue;
        }
        let bytes = load(fixture, path.parent().unwrap())?;
        if let Verification::Pcm(expected) = &fixture.verification {
            let mut machine = run_pcm(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames = option(&args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
            }
            continue;
        }
        if let Verification::Dma(expected) = &fixture.verification {
            let mut machine = run_dma(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames = option(&args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
                if machine.framebuffer().iter().any(|&pixel| pixel != 0x001f) || !machine.halted() {
                    return Err(fail("DMA benchmark scene mismatch"));
                }
            }
            continue;
        }
        if let Verification::Keypad(expected) = &fixture.verification {
            run_keypad(fixture, expected, &bytes)?;
            continue;
        }
        if let Verification::Vblank(expected) = &fixture.verification {
            let mut machine = run_vblank(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames: usize = option(&args, "--frames")?
                    .unwrap_or_else(|| "600".to_owned())
                    .parse()?;
                let before = machine.inspect16(fixture.mailbox_address + 2)?;
                benchmark_frames(fixture, &mut machine, frames)?;
                let count = before.wrapping_add(frames as u16);
                if machine.inspect16(fixture.mailbox_address + 2)? != count
                    || !machine.halted()
                    || machine
                        .framebuffer()
                        .iter()
                        .any(|&pixel| pixel != count.wrapping_sub(1) & 31)
                {
                    return Err(fail("VBlank benchmark output mismatch"));
                }
            }
            continue;
        }
        if let Verification::Sprites(expected) = &fixture.verification {
            let mut machine = run_sprites(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            if bench {
                let point = expected.checkpoints.last().expect("validated checkpoints");
                let b = &point.background;
                if usize::from(point.x) != point.image_x
                    || usize::from(point.y) != point.image_y
                    || point.priority != point.image_priority
                    || point.mapping_1d != point.image_mapping_1d
                    || usize::from(b.x) != b.image_x
                    || usize::from(b.y) != b.image_y
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > machine.cycles().0)
                    || gba_session::BUTTONS
                        .iter()
                        .any(|&button| machine.button_pressed(button))
                {
                    return Err(fail("sprite benchmark requires a settled, released replay"));
                }
                let frames: usize = option(&args, "--frames")?
                    .unwrap_or_else(|| "120".to_owned())
                    .parse()?;
                benchmark_frames(fixture, &mut machine, frames)?;
                check_sprite_image(&machine, point)?;
            }
            continue;
        }
        if let Verification::Tiled(expected) = &fixture.verification {
            let mut machine = run_tiled(
                fixture,
                expected,
                &bytes,
                option(&args, "--capture")?.as_deref(),
            )?;
            if bench {
                let frames: usize = option(&args, "--frames")?
                    .unwrap_or_else(|| "120".to_owned())
                    .parse()?;
                if frames == 0 || frames > 1_000_000 {
                    return Err(fail("frame count must be 1..=1000000"));
                }
                let point = expected.checkpoints.last().expect("validated checkpoints");
                // The replay has released every button. Benchmark stable scanout
                // after initialization and require the same independent image.
                if point.x as usize != point.image_x
                    || point.y as usize != point.image_y
                    || expected
                        .input_events
                        .iter()
                        .any(|event| event.cycle > machine.cycles().0)
                    || gba_session::BUTTONS
                        .iter()
                        .any(|&button| machine.button_pressed(button))
                {
                    return Err(fail("tiled benchmark requires a settled, released replay"));
                }
                benchmark_frames(fixture, &mut machine, frames)?;
                check_tiled_image(&machine, point.image_x, point.image_y)?;
            }
            continue;
        }
        if let Verification::Stripes(expected) = &fixture.verification {
            if !bench {
                run_stripes(
                    fixture,
                    expected,
                    &bytes,
                    option(&args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Diagnostic(expected) = &fixture.verification {
            if !bench {
                run_diagnostic(
                    fixture,
                    &bytes,
                    expected,
                    option(&args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
        if let Verification::Timing(expected) = &fixture.verification {
            verify_timing(fixture, &bytes, expected)?;
            continue;
        }
        if let Verification::Counter(expected) = &fixture.verification {
            if !bench {
                run_counter(
                    fixture,
                    expected,
                    &bytes,
                    option(&args, "--capture")?.as_deref(),
                )?;
            }
            continue;
        }
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
