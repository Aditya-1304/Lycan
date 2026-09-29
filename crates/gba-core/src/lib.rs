#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::ops::Range;

/// Nominal ARM7TDMI clock rate used by the emulated hardware timeline.
pub const GBA_CLOCK_HZ: u64 = 16_777_216;
/// Number of hardware cycles in one GBA scanline.
pub const CYCLES_PER_SCANLINE: u64 = 1_232;
/// Total scanlines in one GBA frame, including vertical blanking.
pub const SCANLINES_PER_FRAME: u64 = 228;
/// Total hardware cycles in one complete GBA frame.
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;
/// Horizontal resolution of the GBA display.
pub const SCREEN_WIDTH: usize = 240;
/// Vertical resolution of the GBA display.
pub const SCREEN_HEIGHT: usize = 160;
/// Number of 16-bit pixels in one GBA framebuffer.
pub const FRAMEBUFFER_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

const ROM_START: u32 = 0x0800_0000;
const MAX_ROM_BYTES: usize = 32 * 1024 * 1024;
const EWRAM_START: u32 = 0x0200_0000;
const IWRAM_START: u32 = 0x0300_0000;
const IO_START: u32 = 0x0400_0000;
const IO_BYTES: usize = 0x400;
const VRAM_START: u32 = 0x0600_0000;
const VRAM_BYTES: usize = 96 * 1024;

const CPSR_N: u32 = 1 << 31;
const CPSR_Z: u32 = 1 << 30;
const CPSR_C: u32 = 1 << 29;
const CPSR_V: u32 = 1 << 28;

const CPSR_I: u32 = 1 << 7;
const CPSR_F: u32 = 1 << 6;
const CPSR_T: u32 = 1 << 5;

const CPSR_SYSTEM_MODE: u32 = 0x1F;

const CONTROLLED_START_CPSR: u32 = CPSR_I | CPSR_F | CPSR_SYSTEM_MODE;

const CONTROLLED_START_SP: u32 = 0x0300_7F00;

/// An absolute position on the emulated hardware cycle timeline.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycle(pub u64);

/// A failure produced while loading or executing a guest program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    EmptyRom,
    RomTooLarge { size: usize, maximum: usize },
    InvalidAccessAlignment { address: u32, width: usize },
    UnmappedAddress { address: u32, width: usize },
    UnsupportedInstruction { address: u32, instruction: u32 },
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRom => formatter.write_str("cannot load an empty ROM"),
            Self::RomTooLarge { size, maximum } => {
                write!(
                    formatter,
                    "ROM is {size} bytes; the supported maximum is {maximum}"
                )
            }
            Self::InvalidAccessAlignment { address, width } => {
                write!(
                    formatter,
                    "unaligned {width}-byte access at {address:#010x}"
                )
            }
            Self::UnmappedAddress { address, width } => {
                write!(formatter, "unmapped {width}-byte access at {address:#010x}")
            }
            Self::UnsupportedInstruction {
                address,
                instruction,
            } => write!(
                formatter,
                "unsupported ARM instruction {instruction:#010x} at {address:#010x}"
            ),
        }
    }
}

impl Error for CoreError {}

/// A bounded guest run either reaches its terminal branch or reports why it stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    Core(CoreError),
    CycleLimitExceeded {
        limit: Cycle,
        reached: Cycle,
        last_pc: u32,
    },
    StepLimitExceeded {
        limit: usize,
        last_pc: u32,
        cycles: Cycle,
    },
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::CycleLimitExceeded {
                limit,
                reached,
                last_pc,
            } => write!(
                formatter,
                "guest exceeded cycle limit {} at cycle {}, PC {last_pc:#010x}",
                limit.0, reached.0
            ),
            Self::StepLimitExceeded {
                limit,
                last_pc,
                cycles,
            } => write!(
                formatter,
                "guest exhausted its {limit}-instruction budget at cycle {}; PC is {last_pc:#010x}",
                cycles.0
            ),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::StepLimitExceeded { .. } | Self::CycleLimitExceeded { .. } => None,
        }
    }
}

impl From<CoreError> for RunError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

/// Actual cycles and last executed address from a bounded run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub instructions: usize,
    pub instruction_address: u32,
    pub cycles: Cycle,
}

/// Access classification remains explicit at the CPU/bus boundary.
#[derive(Clone, Copy)]
enum AccessKind {
    Fetch,
    Data,
}

#[derive(Clone, Copy)]
struct Access {
    kind: AccessKind,
    sequential: bool,
}

trait CpuBus {
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError>;

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError>;

    fn idle(&mut self, cycles: u64);
}

struct Cpu {
    registers: [u32; 16],
    pipeline: [u32; 2],
    pipeline_valid: bool,
    next_fetch_is_sequential: bool,
    cpsr: u32,
}

impl Cpu {
    fn new() -> Self {
        let mut registers = [0; 16];

        registers[13] = CONTROLLED_START_SP;
        registers[15] = ROM_START;

        Self {
            registers,
            pipeline: [0; 2],
            pipeline_valid: false,
            next_fetch_is_sequential: true,
            cpsr: CONTROLLED_START_CPSR,
        }
    }

    /// Fills the two-stage instruction pipeline after reset or a taken branch.
    fn refill<B: CpuBus>(&mut self, bus: &mut B) -> Result<(), CoreError> {
        let pc = self.registers[15];
        self.pipeline[0] = bus.read32(
            pc,
            Access {
                kind: AccessKind::Fetch,
                sequential: false,
            },
        )?;
        self.pipeline[1] = bus.read32(
            pc.wrapping_add(4),
            Access {
                kind: AccessKind::Fetch,
                sequential: true,
            },
        )?;
        self.pipeline_valid = true;
        self.next_fetch_is_sequential = true;
        Ok(())
    }

    /// Executes from the pipeline and accounts for fetch, data, and internal cycles once.
    fn step<B: CpuBus>(&mut self, bus: &mut B) -> Result<StepOutcome, CoreError> {
        if !self.pipeline_valid {
            self.refill(bus)?;
        }
        let address = self.registers[15];
        let instruction = self.pipeline[0];
        let next = bus.read32(
            address.wrapping_add(8),
            Access {
                kind: AccessKind::Fetch,
                sequential: self.next_fetch_is_sequential,
            },
        )?;
        self.pipeline = [self.pipeline[1], next];
        self.registers[15] = address.wrapping_add(4);
        self.next_fetch_is_sequential = true;
        if !self.condition_passes(instruction >> 28) {
            return Ok(StepOutcome { address });
        }
        if instruction & 0x0F00_0000 == 0x0A00_0000 {
            let offset = ((instruction & 0x00FF_FFFF) << 8) as i32 >> 6;
            let target = address.wrapping_add(8).wrapping_add(offset as u32);
            self.registers[15] = target;
            self.refill(bus)?;
        } else if self.execute_data_processing(address, instruction) {
            // Immediate ALU operations use only the pipeline fetch cycle.
        } else if instruction & 0x0F7F_0000 == 0x051F_0000 && (instruction >> 12) & 15 != 15 {
            let offset = instruction & 0xFFF;
            let base = address.wrapping_add(8);
            let target = if instruction & (1 << 23) != 0 {
                base.wrapping_add(offset)
            } else {
                base.wrapping_sub(offset)
            };
            let destination = ((instruction >> 12) & 15) as usize;
            self.registers[destination] = bus.read32(
                target,
                Access {
                    kind: AccessKind::Data,
                    sequential: false,
                },
            )?;
            bus.idle(1);
            self.next_fetch_is_sequential = false;
        } else if self.execute_store_halfword(address, instruction, bus)? {
            self.next_fetch_is_sequential = false;
        } else {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }
        Ok(StepOutcome { address })
    }

    fn condition_passes(&self, condition: u32) -> bool {
        let n = self.cpsr & CPSR_N != 0;
        let z = self.cpsr & CPSR_Z != 0;
        let c = self.cpsr & CPSR_C != 0;
        let v = self.cpsr & CPSR_V != 0;

        match condition {
            0 => z,
            1 => !z,
            2 => c,
            3 => !c,
            4 => n,
            5 => !n,
            6 => v,
            7 => !v,
            8 => c && !z,
            9 => !c || z,
            10 => n == v,
            11 => n != v,
            12 => !z && n == v,
            13 => z || n != v,
            14 => true,
            _ => false,
        }
    }

    /// Implements only the MOV/ORR and SUBS immediate forms required by pixels.s.
    fn execute_data_processing(&mut self, address: u32, instruction: u32) -> bool {
        let opcode = instruction & 0x0FF0_0000;
        if !matches!(opcode, 0x03A0_0000 | 0x0380_0000 | 0x0250_0000) {
            return false;
        }
        let destination = ((instruction >> 12) & 15) as usize;
        if destination == 15 {
            return false;
        }
        let source = ((instruction >> 16) & 15) as usize;
        let lhs = if source == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[source]
        };
        let operand = (instruction & 255).rotate_right(((instruction >> 8) & 15) * 2);
        self.registers[destination] = match opcode {
            0x03A0_0000 => operand,
            0x0380_0000 => lhs | operand,
            _ => {
                let result = lhs.wrapping_sub(operand);

                self.cpsr &= !(CPSR_N | CPSR_Z | CPSR_C | CPSR_V);

                if result & 0x8000_0000 != 0 {
                    self.cpsr |= CPSR_N;
                }

                if result == 0 {
                    self.cpsr |= CPSR_Z;
                }

                if lhs >= operand {
                    self.cpsr |= CPSR_C;
                }

                if ((lhs ^ operand) & (lhs ^ result) & 0x8000_0000) != 0 {
                    self.cpsr |= CPSR_V;
                }

                result
            }
        };
        true
    }

    /// Routes immediate STRH pre/post-indexed stores through the shared timed bus.
    fn execute_store_halfword<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E50_00F0 != 0x0040_00B0 {
            return Ok(false);
        }
        let pre = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0;
        let base_register = ((instruction >> 16) & 15) as usize;
        let source = ((instruction >> 12) & 15) as usize;
        if source == 15 || ((!pre || writeback) && base_register == 15) || (!pre && writeback) {
            return Ok(false);
        }
        let base = if base_register == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[base_register]
        };
        let offset = ((instruction >> 4) & 0xF0) | (instruction & 15);
        let updated = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        bus.write16(
            if pre { updated } else { base },
            self.registers[source] as u16,
            Access {
                kind: AccessKind::Data,
                sequential: false,
            },
        )?;
        if !pre || writeback {
            self.registers[base_register] = updated;
        }
        Ok(true)
    }
}

struct StepOutcome {
    address: u32,
}

struct System {
    rom: Vec<u8>,
    ewram: Vec<u8>,
    iwram: Vec<u8>,
    io: Vec<u8>,
    vram: Vec<u8>,
    display: Display,
    cycles: u64,
}

impl System {
    fn new() -> Self {
        Self {
            rom: Vec::new(),
            ewram: vec![0; 256 * 1024],
            iwram: vec![0; 32 * 1024],
            io: vec![0; IO_BYTES],
            vram: vec![0; VRAM_BYTES],
            display: Display::new(),
            cycles: 0,
        }
    }

    fn read32_impl(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn write16_impl(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }

        self.charge(address, 2, access);

        let bytes = value.to_le_bytes();

        if let Some(range) = range_for(address, EWRAM_START, self.ewram.len(), 2) {
            self.ewram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = range_for(address, IWRAM_START, self.iwram.len(), 2) {
            self.iwram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = range_for(address, IO_START, self.io.len(), 2) {
            let offset = range.start;

            self.io[range].copy_from_slice(&bytes);

            if offset == 0 {
                self.display.control = value;
            }

            return Ok(());
        }

        if let Some(range) = range_for(address, VRAM_START, self.vram.len(), 2) {
            self.vram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        Err(CoreError::UnmappedAddress { address, width: 2 })
    }

    fn read_bytes(&self, address: u32, width: usize) -> Result<&[u8], CoreError> {
        if let Some(range) = range_for(address, ROM_START, self.rom.len(), width) {
            return Ok(&self.rom[range]);
        }
        if let Some(range) = range_for(address, EWRAM_START, self.ewram.len(), width) {
            return Ok(&self.ewram[range]);
        }
        if let Some(range) = range_for(address, IWRAM_START, self.iwram.len(), width) {
            return Ok(&self.iwram[range]);
        }
        if let Some(range) = range_for(address, IO_START, self.io.len(), width) {
            return Ok(&self.io[range]);
        }
        if let Some(range) = range_for(address, VRAM_START, self.vram.len(), width) {
            return Ok(&self.vram[range]);
        }

        Err(CoreError::UnmappedAddress { address, width })
    }

    /// Charges default GBA bus waitstates. WAITCNT/prefetch and video contention
    /// refinements are deferred until a guest fixture demonstrates their need.
    fn charge(&mut self, address: u32, width: usize, access: Access) {
        let _classification = access.kind;
        let beats = (width / 2).max(1) as u64;
        let cycles =
            if address >= ROM_START && (address as u64) < ROM_START as u64 + MAX_ROM_BYTES as u64 {
                let sequential = access.sequential && address & 0x1FFFF != 0;
                (if sequential { 3 } else { 5 }) + if width == 4 { 3 } else { 0 }
            } else if range_for(address, EWRAM_START, 256 * 1024, 1).is_some() {
                3 * beats
            } else if range_for(address, IWRAM_START, 32 * 1024, 1).is_some()
                || range_for(address, IO_START, IO_BYTES, 1).is_some()
            {
                1
            } else {
                beats
            };
        self.advance_time(cycles);
    }

    /// Advances display events before any access changes the state observed by scanout.
    fn advance_time(&mut self, cycles: u64) {
        let target = self.cycles.saturating_add(cycles);
        self.display.synchronize_to(target, &self.vram);
        self.cycles = target;
    }
}

impl CpuBus for System {
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        self.read32_impl(address, access)
    }

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError> {
        self.write16_impl(address, value, access)
    }

    fn idle(&mut self, cycles: u64) {
        self.advance_time(cycles);
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

/// Display timing is independent of VRAM writes and frontend presentation.
struct Display {
    control: u16,
    drawing: Vec<u16>,
    completed: Vec<u16>,
    generation: u64,
    frame_start: u64,
    line: usize,
    next_event: u64,
}

impl Display {
    fn new() -> Self {
        Self {
            control: 0,
            drawing: vec![0; FRAMEBUFFER_PIXELS],
            completed: vec![0; FRAMEBUFFER_PIXELS],
            generation: 0,
            frame_start: 0,
            line: 0,
            next_event: 960,
        }
    }

    /// Renders each visible scanline at its drawing boundary and publishes at VBlank.
    /// Within-line register effects remain the documented scanline approximation.
    fn synchronize_to(&mut self, target: u64, vram: &[u8]) {
        while self.next_event <= target {
            if self.line < SCREEN_HEIGHT {
                let enabled = self.control & 7 == 3 && self.control & (1 << 10) != 0;
                let forced_blank = self.control & (1 << 7) != 0;
                let start = self.line * SCREEN_WIDTH;
                for index in start..start + SCREEN_WIDTH {
                    self.drawing[index] = if forced_blank {
                        0x7FFF
                    } else if enabled {
                        u16::from_le_bytes([vram[index * 2], vram[index * 2 + 1]]) & 0x7FFF
                    } else {
                        0
                    };
                }
                self.line += 1;
                self.next_event = self.frame_start
                    + self.line as u64 * CYCLES_PER_SCANLINE
                    + if self.line == SCREEN_HEIGHT { 0 } else { 960 };
            } else {
                std::mem::swap(&mut self.drawing, &mut self.completed);
                self.generation += 1;
                self.frame_start += CYCLES_PER_FRAME;
                self.line = 0;
                self.next_event = self.frame_start + 960;
            }
        }
    }
}

/// Owns the CPU, memory, display, and cycle state for one emulated machine.
pub struct Machine {
    cpu: Cpu,
    system: System,
    executed_instructions: usize,
}

impl Machine {
    /// Creates an unloaded machine at the GBA cartridge entry address.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a byte-based cartridge image and resets execution to its entry point.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), CoreError> {
        if rom.is_empty() {
            return Err(CoreError::EmptyRom);
        }
        if rom.len() > MAX_ROM_BYTES {
            return Err(CoreError::RomTooLarge {
                size: rom.len(),
                maximum: MAX_ROM_BYTES,
            });
        }

        *self = Self::new();
        self.system.rom.extend_from_slice(rom);
        Ok(())
    }

    /// Executes one instruction and returns its guest instruction address.
    pub fn step(&mut self) -> Result<u32, CoreError> {
        let outcome = self.cpu.step(&mut self.system)?;
        self.executed_instructions = self.executed_instructions.saturating_add(1);
        Ok(outcome.address)
    }

    /// Advances toward an absolute cycle deadline with a hard instruction budget.
    /// An instruction may finish beyond the deadline; subsequent targets retain that excess.
    pub fn advance_to(
        &mut self,
        target: Cycle,
        instruction_limit: usize,
    ) -> Result<RunReport, RunError> {
        let initial = self.executed_instructions;
        let mut last_pc = self.cpu.registers[15];
        while self.cycles() < target {
            if self.executed_instructions - initial >= instruction_limit {
                return Err(RunError::StepLimitExceeded {
                    limit: instruction_limit,
                    last_pc,
                    cycles: self.cycles(),
                });
            }
            last_pc = self.step()?;
        }
        Ok(RunReport {
            instructions: self.executed_instructions - initial,
            instruction_address: last_pc,
            cycles: self.cycles(),
        })
    }

    /// Executes a declared terminal instruction; arbitrary guest loops are not completion.
    pub fn run_until_pc(
        &mut self,
        terminal_pc: u32,
        instruction_limit: usize,
        cycle_limit: Cycle,
    ) -> Result<RunReport, RunError> {
        let initial = self.executed_instructions;
        for _ in 0..instruction_limit {
            let pc = self.step()?;
            if self.cycles() > cycle_limit {
                return Err(RunError::CycleLimitExceeded {
                    limit: cycle_limit,
                    reached: self.cycles(),
                    last_pc: pc,
                });
            }
            if pc == terminal_pc {
                return Ok(RunReport {
                    instructions: self.executed_instructions - initial,
                    instruction_address: pc,
                    cycles: self.cycles(),
                });
            }
        }
        Err(RunError::StepLimitExceeded {
            limit: instruction_limit,
            last_pc: self.cpu.registers[15],
            cycles: self.cycles(),
        })
    }

    /// Reads diagnostic RAM without changing bus timing or device state.
    pub fn inspect16(&self, address: u32) -> Result<u16, CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }

        let bytes = self.system.read_bytes(address, 2)?;

        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Generation of the last completed frame, independent of UI redraws.
    pub fn framebuffer_generation(&self) -> u64 {
        self.system.display.generation
    }

    /// Returns the last completed 16-bit framebuffer in row-major order.
    pub fn framebuffer(&self) -> &[u16] {
        &self.system.display.completed
    }

    /// Returns the current absolute hardware cycle position.
    pub fn cycles(&self) -> Cycle {
        Cycle(self.system.cycles)
    }

    /// Returns the number of successfully executed guest instructions since reset.
    pub fn executed_instructions(&self) -> usize {
        self.executed_instructions
    }

    /// Resets CPU, memory, and display state while retaining the loaded ROM bytes.
    pub fn reset(&mut self) {
        let rom = std::mem::take(&mut self.system.rom);
        *self = Self::new();
        self.system.rom = rom;
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            cpu: Cpu::new(),
            system: System::new(),
            executed_instructions: 0,
        }
    }
}

fn range_for(address: u32, start: u32, length: usize, width: usize) -> Option<Range<usize>> {
    let offset = address.checked_sub(start)? as usize;
    let end = offset.checked_add(width)?;
    (end <= length).then_some(offset..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_stores_are_published_only_after_display_scanout() {
        let mut machine = Machine::new();
        let rom: Vec<u8> = [
            0xE3A00640u32,
            0xE3A01003,
            0xE3811B01,
            0xE1C010B0,
            0xE3A02660,
            0xE3A0301F,
            0xE1C230B0,
            0xEAFFFFFE,
            0,
            0,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();

        machine.load_rom(&rom).unwrap();
        let report = machine
            .run_until_pc(ROM_START + 28, 64, Cycle(1000))
            .unwrap();

        assert_eq!(report.instructions, 8);
        // A VRAM write is not a completed frame. Presentation must wait for scanout.
        assert_eq!(machine.framebuffer()[0], 0);
        assert_eq!(machine.framebuffer_generation(), 0);
        let report = machine
            .advance_to(Cycle(CYCLES_PER_FRAME), 100_000)
            .unwrap();
        assert!(report.cycles.0 >= CYCLES_PER_FRAME);
        assert_eq!(machine.framebuffer_generation(), 1);
        assert_eq!(machine.framebuffer()[0], 0x001F);
        // A guest that keeps looping still respects a caller's finite work budget.
        assert!(matches!(
            machine.advance_to(Cycle(2 * CYCLES_PER_FRAME), 1),
            Err(RunError::StepLimitExceeded { .. })
        ));
    }

    #[test]
    fn controlled_startup_is_arm_system_with_interrupts_masked() {
        let machine = Machine::new();

        assert_eq!(machine.cpu.registers[13], 0x0300_7F00);

        assert_eq!(machine.cpu.cpsr & 0xFF, CONTROLLED_START_CPSR);

        assert_eq!(machine.cpu.cpsr & CPSR_T, 0);
        assert_ne!(machine.cpu.cpsr & CPSR_I, 0);
        assert_ne!(machine.cpu.cpsr & CPSR_F, 0);
        assert_eq!(machine.cpu.cpsr & 0x1F, CPSR_SYSTEM_MODE);
    }
}
