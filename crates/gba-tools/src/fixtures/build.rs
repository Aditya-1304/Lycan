//! Assembly fixture rebuilding with frozen ROM and firmware identity checks.

use super::manifest::{Verification, load, manifest};
use crate::support::{Result, fail, hash, root};
use std::fs;
use std::path::Path;
use std::process::Command;

pub(super) fn tool(program: &str, args: &[&std::ffi::OsStr]) -> Result<()> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(fail(format!("{program} failed with {status}")));
    }
    Ok(())
}

/// Rebuilds original assembly with GNU Arm binutils and verifies its frozen identity.
pub(crate) fn build_fixtures(path: &Path) -> Result<()> {
    let manifest = manifest(path)?;
    let directory = path
        .parent()
        .ok_or_else(|| fail("fixture manifest has no parent directory"))?;
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
                root()
                    .join("fixtures/diagnostics/test-firmware/division.s")
                    .as_os_str(),
            ],
        )?;
        tool(
            "arm-none-eabi-ld",
            &[
                "-T".as_ref(),
                root()
                    .join("fixtures/diagnostics/test-firmware/linker.ld")
                    .as_os_str(),
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
        fs::write(
            root().join("fixtures/diagnostics/test-firmware/division.bin"),
            bytes,
        )?;
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
