//! Minimal runtime bridge to the production core. Expectations are supplied
//! from the frozen manifest by the Node runner, never copied into this module.

use gba_core::{Cycle, Machine};
use std::cell::RefCell;

thread_local! {
    /// One owned guest machine survives successive marker calls in this instance.
    static MACHINE: RefCell<Machine> = RefCell::new(Machine::new());
}

/// Loads the hash-checked guest embedded by the verification driver.
#[unsafe(no_mangle)]
pub extern "C" fn initialize(firmware: u32) {
    MACHINE.with_borrow_mut(|machine| {
        machine
            .load_rom(include_bytes!(env!("GBA_VERIFICATION_ROM")))
            .unwrap();
        if firmware != 0 {
            machine.enable_test_firmware();
        }
    });
}

/// Executes through a declared instruction address under the shared total
/// budgets. Panics trap the WASM instance and cause the Node process to fail.
#[unsafe(no_mangle)]
pub extern "C" fn marker(pc: u32, instructions: u32, cycles: u64) -> u64 {
    MACHINE.with_borrow_mut(|machine| {
        let remaining = (instructions as usize)
            .checked_sub(machine.executed_instructions())
            .expect("instruction budget exhausted");
        machine.run_until_pc(pc, remaining, Cycle(cycles)).unwrap();
        assert!(machine.executed_instructions() <= instructions as usize);
        assert!(machine.cycles().0 <= cycles);
        machine.cycles().0
    })
}

/// Reads observable state without changing bus timing or device state.
#[unsafe(no_mangle)]
pub extern "C" fn inspect16(address: u32) -> u32 {
    MACHINE.with_borrow(|machine| u32::from(machine.inspect16(address).unwrap()))
}

/// Exposes architectural diagnostic state, distinct from the pipeline PC.
#[unsafe(no_mangle)]
pub extern "C" fn register(index: u32) -> u32 {
    MACHINE.with_borrow(|machine| machine.registers()[index as usize])
}

/// Exposes the CPU status required by the frozen diagnostic completion contract.
#[unsafe(no_mangle)]
pub extern "C" fn cpsr() -> u32 {
    MACHINE.with_borrow(|machine| machine.cpsr())
}

/// Reports total executed instructions for bounded diagnostic evidence.
#[unsafe(no_mangle)]
pub extern "C" fn instructions() -> u32 {
    MACHINE.with_borrow(|machine| machine.executed_instructions() as u32)
}

/// Completes scanout within the same total budgets as terminal execution.
#[unsafe(no_mangle)]
pub extern "C" fn advance(target: u64, instructions: u32, cycles: u64) {
    assert!(target <= cycles);
    MACHINE.with_borrow_mut(|machine| {
        let remaining = (instructions as usize)
            .checked_sub(machine.executed_instructions())
            .expect("scanout instruction budget exhausted");
        machine.advance_to(Cycle(target), remaining).unwrap();
        assert!(machine.executed_instructions() <= instructions as usize);
        assert!(machine.cycles().0 <= cycles);
    });
}

/// Reads completed guest pixels for a host-side SHA-256 comparison; it does
/// not synthesize an expected image or modify the framebuffer.
#[unsafe(no_mangle)]
pub extern "C" fn pixel(index: u32) -> u32 {
    MACHINE.with_borrow(|machine| u32::from(machine.framebuffer()[index as usize]))
}
