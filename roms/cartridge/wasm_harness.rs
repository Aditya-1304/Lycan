//! Minimal runtime bridge to the production core. Expectations are supplied
//! from the frozen manifest by the Node runner, never copied into this module.

use gba_core::{Button, Cycle, Machine};
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

/// Reloads a declared guest configuration variant for interrupt gating checks.
/// Patching affects guest ROM data only; interrupt delivery still uses the core.
#[unsafe(no_mangle)]
pub extern "C" fn initialize_variant(offset: u32, configuration: u32, firmware: u32) {
    let mut bytes = include_bytes!(env!("GBA_VERIFICATION_ROM")).to_vec();
    bytes[offset as usize..offset as usize + 4].copy_from_slice(&configuration.to_le_bytes());
    MACHINE.with_borrow_mut(|machine| {
        machine.load_rom(&bytes).unwrap();
        if firmware != 0 {
            machine.enable_test_firmware();
        }
    });
}

/// Observes CPU sleep without changing pending flags or dispatching a callback.
#[unsafe(no_mangle)]
pub extern "C" fn halted() -> u32 {
    MACHINE.with_borrow(|machine| u32::from(machine.halted()))
}

/// Keeps a deliberately unmapped-vector probe recoverable so its mailbox can
/// be checked after rejection; normal contract failures still trap in advance.
#[unsafe(no_mangle)]
pub extern "C" fn rejects_advance(target: u64, instructions: u32) -> u32 {
    MACHINE.with_borrow_mut(|machine| {
        u32::from(
            machine
                .advance_to(Cycle(target), instructions as usize)
                .is_err(),
        )
    })
}

/// Queue the exact logical transitions shared by the native runner and app.
/// Scheduling is handled by the production core, including HALT deadlines.
#[unsafe(no_mangle)]
pub extern "C" fn queue_keypad_input() {
    const INPUT: &[(Cycle, Button, bool)] = &include!("../keypad/input.rs");
    MACHINE.with_borrow_mut(|machine| {
        for &(cycle, button, pressed) in INPUT {
            machine.set_button_at(cycle, button, pressed).unwrap();
        }
    });
}

/// Expose hardware time and completed scanout for exact input/frame assertions.
#[unsafe(no_mangle)]
pub extern "C" fn cycles() -> u64 {
    MACHINE.with_borrow(|machine| machine.cycles().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn generation() -> u64 {
    MACHINE.with_borrow(|machine| machine.framebuffer_generation())
}

/// Queues the DMA scene timeline shared with the native runner and application.
#[unsafe(no_mangle)]
pub extern "C" fn queue_dma_input() {
    use gba_core::CYCLES_PER_FRAME;
    const INPUT: &[(Cycle, Button, bool)] = &include!("../dma/input.rs");
    MACHINE.with_borrow_mut(|machine| {
        for &(cycle, button, pressed) in INPUT {
            machine.set_button_at(cycle, button, pressed).unwrap();
        }
    });
}


thread_local! {
    /// Owned staging for the Node PCM hash comparison, not a playback adapter.
    static PCM: RefCell<Vec<f32>> = RefCell::new(Vec::with_capacity(1024));
}

/// Queues the same PCM-scene input chronology used by the app and native runner.
#[unsafe(no_mangle)]
pub extern "C" fn queue_pcm_input() {
    use gba_core::CYCLES_PER_FRAME;
    const INPUT: &[(Cycle, Button, bool)] = &include!("../pcm/input.rs");
    MACHINE.with_borrow_mut(|machine| {
        for &(cycle, button, pressed) in INPUT {
            machine.set_button_at(cycle, button, pressed).unwrap();
        }
    });
}

/// Drains real production-core samples for signed-byte fixture hashing.
#[unsafe(no_mangle)]
pub extern "C" fn drain_pcm() -> u32 {
    PCM.with_borrow_mut(|samples| {
        samples.clear();
        MACHINE.with_borrow_mut(|machine| machine.drain_pcm(samples));
        samples.len() as u32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pcm_byte(index: u32) -> u32 {
    PCM.with_borrow(|samples| (samples[index as usize] * 128.0) as i8 as u8 as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn pcm_counter(index: u32) -> u64 {
    MACHINE.with_borrow(|machine| {
        let (produced, dropped, empty) = machine.pcm_counters();
        match index { 0 => produced, 1 => dropped, 2 => empty, _ => panic!("invalid PCM counter") }
    })
}


thread_local! {
    /// Stereo staging retains signed DAC units for native/WASM contract parity.
    static STEREO_PCM: RefCell<Vec<[f32; 2]>> = RefCell::new(Vec::with_capacity(1024));
}

/// Continues the synchronized scene through the same guest mixer input replay.
#[unsafe(no_mangle)]
pub extern "C" fn queue_mixer_input() {
    use gba_core::CYCLES_PER_FRAME;
    const INPUT: &[(Cycle, Button, bool)] = &include!("../pcm/mixer_input.rs");
    MACHINE.with_borrow_mut(|machine| {
        for &(cycle, button, pressed) in INPUT { machine.set_button_at(cycle, button, pressed).unwrap(); }
    });
}

/// Returns stereo frame count; samples remain owned by this WASM instance.
#[unsafe(no_mangle)]
pub extern "C" fn drain_stereo_pcm() -> u32 {
    STEREO_PCM.with_borrow_mut(|samples| {
        samples.clear();
        MACHINE.with_borrow_mut(|machine| machine.drain_stereo_pcm(samples));
        samples.len() as u32
    })
}

/// Reads one signed DAC level without exposing or transferring linear memory.
#[unsafe(no_mangle)]
pub extern "C" fn stereo_level(index: u32, channel: u32) -> i32 {
    STEREO_PCM.with_borrow(|samples| (samples[index as usize][channel as usize] * 512.0) as i32)
}
