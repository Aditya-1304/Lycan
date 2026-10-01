//! Shared native/WASM guest contract for the pinned upstream BIOS diagnostic.
//! Firmware bytes are supplied at runtime and are never embedded in artifacts.
use gba_core::{CYCLES_PER_FRAME, Cycle, Machine};

pub const ROM: &[u8] = include_bytes!("../gba-tests/bios/bios.gba");
pub const TERMINAL_PC: u32 = 0x080003c0;
pub const MAX_CYCLES: u64 = 600 * CYCLES_PER_FRAME;
pub const MAX_INSTRUCTIONS: usize = 50_000_000;

/// Executes BIOS reset, sqrt SWI, and a real VBlank IRQ before checking r12.
/// Reaching any other idle loop cannot satisfy this completion contract.
pub fn verify(bios: &[u8]) -> Result<Machine, String> {
    let mut machine = Machine::new();
    machine.load_bios(bios).map_err(|error| error.to_string())?;
    machine
        .boot_rom_with_bios(ROM, None)
        .map_err(|error| error.to_string())?;
    machine
        .run_until_pc(TERMINAL_PC, MAX_INSTRUCTIONS, Cycle(MAX_CYCLES))
        .map_err(|error| {
            format!(
                "BIOS diagnostic PC={:#010x}: {error}",
                machine.registers()[15]
            )
        })?;
    if machine.registers()[12] != 0 {
        return Err(format!("bios.gba failed test {}", machine.registers()[12]));
    }
    let target = Cycle((machine.cycles().0 / CYCLES_PER_FRAME + 2) * CYCLES_PER_FRAME);
    if target.0 > MAX_CYCLES {
        return Err("no scanout budget remains".to_owned());
    }
    machine
        .advance_to(
            target,
            MAX_INSTRUCTIONS.saturating_sub(machine.executed_instructions()),
        )
        .map_err(|error| error.to_string())?;
    Ok(machine)
}
