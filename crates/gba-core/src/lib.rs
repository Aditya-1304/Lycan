#![forbid(unsafe_code)]

pub mod fixtures;

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
    StepLimitExceeded { limit: usize, last_pc: u32 },
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::StepLimitExceeded { limit, last_pc } => write!(
                formatter,
                "guest did not reach its terminal branch within {limit} instructions; PC is {last_pc:#010x}"
            ),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::StepLimitExceeded { .. } => None,
        }
    }
}

impl From<CoreError> for RunError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

/// Summary of a guest program that reached its self-branch terminal point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub instructions: usize,
    pub terminal_pc: u32,
    pub cycles: Cycle,
}

struct Cpu {
    registers: [u32; 16],
    next_fetch_is_sequential: bool,
}

impl Cpu {
    fn new() -> Self {
        let mut cpu = Self::default();
        cpu.registers[15] = ROM_START;
        cpu
    }

    /// Executes one ARM instruction using the separately borrowed system bus.
    fn step(&mut self, system: &mut System) -> Result<StepOutcome, CoreError> {
        let address = self.registers[15];
        let instruction = system.read32(address, self.next_fetch_is_sequential)?;
        self.registers[15] = address.wrapping_add(4);

        if instruction >> 28 != 0xE {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }

        let branch_target = if instruction & 0x0E00_0000 == 0x0A00_0000 {
            let signed_offset = ((instruction & 0x00FF_FFFF) << 8) as i32 >> 6;
            let target = (address as i32).wrapping_add(8).wrapping_add(signed_offset) as u32;
            self.registers[15] = target;
            Some(target)
        } else if self.execute_data_processing(address, instruction)? {
            None
        } else if self.execute_load_literal(address, instruction, system)? {
            None
        } else if self.execute_store_halfword(address, instruction, system)? {
            None
        } else {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        };

        // The initial timing model accounts for an internal execution cycle per instruction.
        system.idle(1);
        self.next_fetch_is_sequential = branch_target.is_none();

        Ok(StepOutcome {
            address,
            branch_target,
        })
    }

    /// Executes the immediate MOV and ORR forms used by the first guest fixture.
    fn execute_data_processing(
        &mut self,
        address: u32,
        instruction: u32,
    ) -> Result<bool, CoreError> {
        let opcode = instruction & 0x0FE0_0000;
        if opcode != 0x03A0_0000 && opcode != 0x0380_0000 {
            return Ok(false);
        }

        let destination = ((instruction >> 12) & 0xF) as usize;
        let source = ((instruction >> 16) & 0xF) as usize;
        if destination == 15 {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }

        let immediate = instruction & 0xFF;
        let rotation = ((instruction >> 8) & 0xF) * 2;
        let operand = immediate.rotate_right(rotation);

        self.registers[destination] = if opcode == 0x03A0_0000 {
            operand
        } else {
            self.registers[source] | operand
        };

        Ok(true)
    }

    /// Loads a 32-bit literal from the ARM PC-relative address form.
    fn execute_load_literal(
        &mut self,
        address: u32,
        instruction: u32,
        system: &mut System,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0F7F_0000 != 0x051F_0000 {
            return Ok(false);
        }

        let destination = ((instruction >> 12) & 0xF) as usize;
        if destination == 15 {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }

        let offset = instruction & 0xFFF;
        let base = address.wrapping_add(8);
        let literal_address = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        self.registers[destination] = system.read32(literal_address, false)?;
        Ok(true)
    }

    /// Executes the immediate STRH form used to configure display control and pixels.
    fn execute_store_halfword(
        &self,
        address: u32,
        instruction: u32,
        system: &mut System,
    ) -> Result<bool, CoreError> {
        let halfword_pattern = instruction & 0x0E00_00F0 == 0x0000_00B0;
        let immediate = instruction & (1 << 22) != 0;
        let preindexed = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0;
        let load = instruction & (1 << 20) != 0;

        if !(halfword_pattern && immediate && preindexed && !writeback && !load) {
            return Ok(false);
        }

        let base_register = ((instruction >> 16) & 0xF) as usize;
        let source_register = ((instruction >> 12) & 0xF) as usize;
        if source_register == 15 {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }

        let offset = ((instruction >> 4) & 0xF0) | (instruction & 0xF);
        let base = if base_register == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[base_register]
        };
        let target = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };

        system.write16(target, self.registers[source_register] as u16)?;
        Ok(true)
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            registers: [0; 16],
            next_fetch_is_sequential: false,
        }
    }
}

struct StepOutcome {
    address: u32,
    branch_target: Option<u32>,
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

    fn read32(&mut self, address: u32, sequential: bool) -> Result<u32, CoreError> {
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, sequential);
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn write16(&mut self, address: u32, value: u16) -> Result<(), CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }
        self.charge(address, 2, false);
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
                self.display.set_control(value);
                self.refresh_mode3_framebuffer();
            }
            return Ok(());
        }
        if let Some(range) = range_for(address, VRAM_START, self.vram.len(), 2) {
            let offset = range.start;
            self.vram[range].copy_from_slice(&bytes);
            if offset < FRAMEBUFFER_PIXELS * 2 {
                self.display.write_pixel(offset / 2, value);
            }
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

    fn refresh_mode3_framebuffer(&mut self) {
        if !self.display.mode3_enabled {
            return;
        }

        for pixel_index in 0..FRAMEBUFFER_PIXELS {
            let byte_index = pixel_index * 2;
            let value = u16::from_le_bytes([self.vram[byte_index], self.vram[byte_index + 1]]);
            self.display.write_pixel(pixel_index, value);
        }
    }

    /// Accounts for the starter bus timing model used by this display fixture.
    fn charge(&mut self, address: u32, width: usize, sequential: bool) {
        let halfword_transfers = (width / 2).max(1) as u64;
        let cycles =
            if address >= ROM_START && (address as u64) < ROM_START as u64 + MAX_ROM_BYTES as u64 {
                let first = if sequential { 3 } else { 5 };
                first + if width == 4 { 3 } else { 0 }
            } else if range_for(address, EWRAM_START, 256 * 1024, 1).is_some() {
                3 * halfword_transfers
            } else if range_for(address, IWRAM_START, 32 * 1024, 1).is_some() {
                halfword_transfers
            } else {
                halfword_transfers
            };
        self.cycles = self.cycles.saturating_add(cycles);
    }

    fn idle(&mut self, cycles: u64) {
        self.cycles = self.cycles.saturating_add(cycles);
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

struct Display {
    mode3_enabled: bool,
    framebuffer: Vec<u16>,
}

impl Display {
    fn new() -> Self {
        Self {
            mode3_enabled: false,
            framebuffer: vec![0; FRAMEBUFFER_PIXELS],
        }
    }

    fn set_control(&mut self, value: u16) {
        self.mode3_enabled = value & 0x7 == 3 && value & (1 << 10) != 0;
        if !self.mode3_enabled {
            self.framebuffer.fill(0);
        }
    }

    fn write_pixel(&mut self, index: usize, value: u16) {
        if self.mode3_enabled && index < self.framebuffer.len() {
            self.framebuffer[index] = value & 0x7FFF;
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

    /// Runs a guest program until it branches to itself or exhausts its instruction budget.
    pub fn run_until_self_branch(
        &mut self,
        instruction_limit: usize,
    ) -> Result<RunReport, RunError> {
        let initial_instruction_count = self.executed_instructions;
        for _ in 0..instruction_limit {
            let outcome = self.cpu.step(&mut self.system)?;
            self.executed_instructions = self.executed_instructions.saturating_add(1);
            if outcome.branch_target == Some(outcome.address) {
                return Ok(RunReport {
                    instructions: self.executed_instructions - initial_instruction_count,
                    terminal_pc: outcome.address,
                    cycles: Cycle(self.system.cycles),
                });
            }
        }

        Err(RunError::StepLimitExceeded {
            limit: instruction_limit,
            last_pc: self.cpu.registers[15],
        })
    }

    /// Returns the guest's current 16-bit framebuffer in row-major order.
    pub fn framebuffer(&self) -> &[u16] {
        &self.system.display.framebuffer
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
    fn executed_pixels_rom_writes_the_expected_mode3_colors() {
        let mut machine = Machine::new();
        let rom = fixtures::pixels_gba();

        machine.load_rom(&rom).unwrap();
        let report = machine.run_until_self_branch(64).unwrap();

        assert_eq!(report.instructions, 28);
        assert_eq!(
            &machine.framebuffer()[..8],
            &[
                0x001F, 0x03E0, 0x7C00, 0x03FF, 0x7C1F, 0x7FE0, 0x7FFF, 0x0000
            ]
        );
    }
}
