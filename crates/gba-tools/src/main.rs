use std::error::Error;
use std::io;

use gba_core::Machine;
use gba_core::fixtures::pixels_gba;

const PIXELS_INSTRUCTION_LIMIT: usize = 64;
const EXPECTED_SAMPLES: [u16; 8] = [
    0x001F, 0x03E0, 0x7C00, 0x03FF, 0x7C1F, 0x7FE0, 0x7FFF, 0x0000,
];

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

/// Executes the controlled-startup pixels ROM and checks its exact guest output.
fn run() -> Result<(), Box<dyn Error>> {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "pixels".to_owned());
    if command != "pixels" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command {command:?}; supported command: pixels"),
        )
        .into());
    }

    let rom = pixels_gba();
    let mut machine = Machine::new();
    machine.load_rom(&rom)?;
    let report = machine.run_until_self_branch(PIXELS_INSTRUCTION_LIMIT)?;

    let actual = &machine.framebuffer()[..EXPECTED_SAMPLES.len()];
    if actual != EXPECTED_SAMPLES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pixels.gba framebuffer mismatch: expected {EXPECTED_SAMPLES:04x?}, got {actual:04x?}"),
        )
        .into());
    }

    let samples = actual
        .iter()
        .map(|pixel| format!("{pixel:04x}"))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "PASS pixels.gba pixels={samples} instructions={} cycles={}",
        report.instructions, report.cycles.0
    );
    Ok(())
}
