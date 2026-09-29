#![forbid(unsafe_code)]

pub const GBA_CLOCK_HZ: u64 = 16_777_216;
pub const CYCLES_PER_SCANLINE: u64 = 1_232;
pub const SCANLINES_PER_FRAME: u64 = 228;
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycle(pub u64);

trait CpuBus {}

#[derive(Default)]
struct Cpu {
    steps: u64,
}

impl Cpu {
    fn step<B: CpuBus>(&mut self, _bus: &mut B) {
        self.steps += 1;
    }
}

#[derive(Default)]
struct System;

impl CpuBus for System {}

#[derive(Default)]
pub struct Machine {
    cpu: Cpu,
    system: System,
}

impl Machine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self) {
        let Self { cpu, system } = self;
        cpu.step(system);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

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
