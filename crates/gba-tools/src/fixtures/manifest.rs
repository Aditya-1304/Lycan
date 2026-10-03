//! Frozen fixture schemas, input timelines, configuration validation, and ROM identity.

use crate::support::{Result, button, fail, hash};
use gba_core::{CYCLES_PER_FRAME, Cycle, SCREEN_HEIGHT, SCREEN_WIDTH};
use gba_session::Button;
use serde::Deserialize;
use std::{fs, path::Path};

/// App replay input is checked against the independent frozen manifest below.
pub(super) const DEMO_INPUT: &[(Cycle, Button, bool)] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/buttons/input.rs"
));

/// The app replay is checked against the separately frozen fixture timeline.
pub(super) const TILED_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/tiled/input.rs");

/// The application timeline must match independently retained fixture events.
pub(super) const SPRITE_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/sprites/input.rs");
pub(super) const DMA_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/dma/input.rs");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub(super) version: u32,
    pub(super) fixture: Vec<Fixture>,
}

/// The frozen manifest specifies completion independently of the guest implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Fixture {
    pub(super) name: String,
    pub(super) rom: String,
    pub(super) source: String,
    pub(super) linker: String,
    pub(super) sha256: String,
    pub(super) origin: String,
    pub(super) startup: String,
    pub(super) bios_required: bool,
    pub(super) mailbox_address: u32,
    pub(super) completion_id: u16,
    pub(super) max_instructions: usize,
    pub(super) max_cycles: u64,
    pub(super) verification: Verification,
}

/// Each fixture declares its own observable completion rather than relying on
/// an arbitrary guest loop. Moving scenes require bounded checkpoints.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum Verification {
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
pub(super) struct PcmScene {
    pub(super) ready_pc: u32,
    pub(super) firmware_sha256: String,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<PcmCheckpoint>,
    pub(super) mixer_input_events: Vec<InputEvent>,
    pub(super) mixer_checkpoints: Vec<MixerCheckpoint>,
    pub(super) mixer_fifo_underruns: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PcmCheckpoint {
    pub(super) frame: u16,
    pub(super) color: u16,
    pub(super) pressed: u16,
    pub(super) transitions: u16,
    pub(super) samples: usize,
    pub(super) pcm_sha256: String,
}

/// Stereo DAC levels are expressed in signed 10-bit units, before host gain.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MixerCheckpoint {
    pub(super) frame: u16,
    pub(super) control_h: u16,
    pub(super) master: u16,
    pub(super) bias: u16,
    pub(super) levels: Vec<[i16; 2]>,
    pub(super) pcm_sha256: String,
}

pub(super) const MIXER_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/pcm/mixer_input.rs");

pub(super) const PCM_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/pcm/input.rs");

/// The DMA scene has an independently frozen startup PC and firmware identity.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DmaScene {
    pub(super) ready_pc: u32,
    pub(super) firmware_sha256: String,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<DmaCheckpoint>,
}

/// Frozen data, complete scanout and interrupt counts at one frame boundary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DmaCheckpoint {
    pub(super) frame: u16,
    pub(super) tile: u16,
    pub(super) color: u16,
    pub(super) vblank: u16,
    pub(super) completions: u16,
}

/// Frozen keypad mode and mapped firmware identity for bounded guest acceptance.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Keypad {
    pub(super) and_mode: bool,
    pub(super) firmware_sha256: String,
}

pub(super) const KEYPAD_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../../fixtures/diagnostics/keypad/input.rs");

/// IRQ guest identity and bounded completion; configuration variants only change
/// the declared source/master/CPU mask word in the verified original ROM.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Vblank {
    pub(super) configuration_offset: usize,
    pub(super) frames: u16,
    pub(super) firmware_sha256: String,
}

/// Scripted guest offsets and independently computed complete tile-scene captures.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tiled {
    pub(super) ready_pc: u32,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<TiledCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TiledCheckpoint {
    pub(super) frame: u16,
    pub(super) updates: u16,
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) image_x: usize,
    pub(super) image_y: usize,
}

/// Frozen player/OAM state accompanies the independent background checkpoints.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sprites {
    /// Present only for the IRQ scene; the polling fixture leaves firmware unmapped.
    #[serde(default)]
    pub(super) firmware_sha256: Option<String>,
    pub(super) ready_pc: u32,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<SpriteCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpriteCheckpoint {
    pub(super) background: TiledCheckpoint,
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) image_x: usize,
    pub(super) image_y: usize,
    pub(super) priority: u16,
    pub(super) image_priority: u16,
    pub(super) mapping_1d: bool,
    pub(super) image_mapping_1d: bool,
}

/// Upstream success is tied to its source-defined idle PC and exact stripe colors.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stripes {
    pub(super) terminal_pc: u32,
}

/// Frozen diagnostic success requires a specific opcode, status, result, and
/// original firmware identity. An unrelated self-branch never satisfies it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Diagnostic {
    /// Pinned homebrew without an SDK version uses the centralized typed override.
    pub(super) backup_override: Option<String>,
    pub(super) terminal_pc: u32,
    pub(super) terminal_instruction: u32,
    pub(super) cpsr_mask: u32,
    pub(super) cpsr_value: u32,
    pub(super) result_register: usize,
    pub(super) result: u32,
    pub(super) firmware_sha256: String,
    pub(super) framebuffer_sha256: Option<String>,
}

/// Independently derived timing intervals are measured after both marker
/// instructions, including CPU pipeline fetches but excluding setup/reporting.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Timing {
    pub(super) terminal_pc: u32,
    pub(super) checkpoints: Vec<TimingCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TimingCheckpoint {
    pub(super) start_pc: u32,
    pub(super) end_pc: u32,
    pub(super) cycles: u64,
    pub(super) waitcnt: u16,
    pub(super) width: u32,
    pub(super) window: u32,
}

/// Frozen observations bind input, ARM return, Thumb PC semantics, and scanout.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Counter {
    pub(super) return_pc: u32,
    pub(super) adr_address: u32,
    pub(super) observed_pc: u32,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<CounterCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CounterCheckpoint {
    pub(super) frame: u16,
    pub(super) count: u16,
    pub(super) image_count: u16,
}

/// Pinned upstream hello completion and an independently derived full-frame
/// digest. The reference ROM has no original-fixture completion mailbox.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Hello {
    pub(super) terminal_pc: u32,
    pub(super) framebuffer_sha256: String,
}

/// One guest reproduction covers indexed lookup, both pages, and a palette-only
/// change. Frozen colors are independent of the renderer and guest stores.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Palette {
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<PaletteCheckpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PaletteCheckpoint {
    pub(super) frame: u16,
    pub(super) page: u16,
    pub(super) palette_changed: u16,
    pub(super) colors: [u16; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pixels {
    pub(super) terminal_pc: u32,
    pub(super) result: u16,
    pub(super) band_height: usize,
    pub(super) colors: Vec<u16>,
    pub(super) input_events: Vec<String>,
    pub(super) time_events: Vec<String>,
    /// Frozen halfwords checked independently of the displayed frame.
    #[serde(default)]
    pub(super) memory_checks: Vec<MemoryCheck>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MemoryCheck {
    pub(super) address: u32,
    pub(super) value: u16,
    /// Number of consecutive halfwords; singleton probes omit this field.
    #[serde(default = "one_halfword")]
    pub(super) halfwords: u32,
}

pub(super) fn one_halfword() -> u32 {
    1
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Movement {
    pub(super) square_size: usize,
    pub(super) color: u16,
    pub(super) input_events: Vec<InputEvent>,
    pub(super) checkpoints: Vec<Checkpoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InputEvent {
    pub(super) cycle: u64,
    pub(super) button: String,
    pub(super) pressed: bool,
}

/// RAM reports current guest state; scanout contains the preceding frame's
/// drawing because the guest changes VRAM after publication at VBlank.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub(super) frame: u16,
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) image_x: usize,
    pub(super) image_y: usize,
}

pub(super) fn manifest(path: &Path) -> Result<Manifest> {
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

pub(super) fn load(fixture: &Fixture, directory: &Path) -> Result<Vec<u8>> {
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
