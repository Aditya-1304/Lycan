#![forbid(unsafe_code)]

/// Nominal ARM7TDMI clock rate used by the later cycle-based execution model.
pub const GBA_CLOCK_HZ: u64 = 16_777_216;
/// Number of hardware cycles in one GBA scanline.
pub const CYCLES_PER_SCANLINE: u64 = 1_232;
/// Total scanlines in one GBA frame, including vertical blanking.
pub const SCANLINES_PER_FRAME: u64 = 228;
/// Total hardware cycles in one complete GBA frame.
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;

/// An absolute position on the emulated hardware cycle timeline.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycle(pub u64);

/// Operations the CPU can request from its system bus.
///
/// This private boundary keeps CPU execution independent of any host platform.
trait CpuBus {}

/// Temporary CPU state used to establish the Machine ownership boundary in Slice 0.
#[derive(Default)]
struct Cpu {
    steps: u64,
}

impl Cpu {
    /// Records one placeholder CPU step while borrowing the system bus separately.
    fn step<B: CpuBus>(&mut self, _bus: &mut B) {
        self.steps += 1;
    }
}

/// Placeholder for memory-mapped hardware and the CPU-visible system bus.
#[derive(Default)]
struct System;

impl CpuBus for System {}

/// Owns the CPU and all emulated system devices as one resettable machine.
///
/// Slice 0 only establishes ownership and split borrowing; it does not execute GBA
/// instructions or model memory-mapped hardware.
#[derive(Default)]
pub struct Machine {
    cpu: Cpu,
    system: System,
}

impl Machine {
    /// Constructs a machine in its initial state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the placeholder CPU while passing the system bus as a separate borrow.
    pub fn step(&mut self) {
        let Self { cpu, system } = self;
        cpu.step(system);
    }

    /// Restores the machine and its owned components to their initial state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns the number of placeholder CPU steps since the last reset.
    pub fn executed_steps(&self) -> u64 {
        self.cpu.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_uses_split_cpu_system_borrow() {
        let mut machine = Machine::new();

        machine.step();

        assert_eq!(machine.executed_steps(), 1);
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut machine = Machine::new();

        machine.step();
        machine.reset();

        assert_eq!(machine.executed_steps(), 0);
    }
}
