#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::ops::Range;

/// Logical GBA buttons in their KEYINPUT bit order, independent of host keys.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
    R,
    L,
}

/// Pressed-button bitset; conversion to active-low hardware bits stays in the core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState(u16);
impl ButtonState {
    /// Updates one logical button without affecting the other nine buttons.
    pub fn set(&mut self, button: Button, pressed: bool) {
        let mask = 1 << button as u8;
        if pressed {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
    /// Reports whether the logical button is pressed.
    pub fn pressed(self, button: Button) -> bool {
        self.0 & (1 << button as u8) != 0
    }
    /// Releases all logical buttons.
    pub fn release_all(&mut self) {
        self.0 = 0;
    }
}

/// Ordered external input delivered on the same cycle timeline as hardware accesses.
struct InputEvent {
    cycle: Cycle,
    button: Button,
    pressed: bool,
}

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
const PALETTE_START: u32 = 0x0500_0000;
const PALETTE_BYTES: usize = 1024;
const VRAM_START: u32 = 0x0600_0000;
const VRAM_BYTES: usize = 96 * 1024;
// The scanline renderer samples at 960 cycles, but the DISPSTAT HBlank flag
// follows the 1008-cycle mGBA timing model. IRQ edge timing is a later slice.
const HBLANK_FLAG_CYCLE: u64 = 1_008;

const CPSR_N: u32 = 1 << 31;
const CPSR_Z: u32 = 1 << 30;
const CPSR_C: u32 = 1 << 29;
const CPSR_V: u32 = 1 << 28;

const CPSR_I: u32 = 1 << 7;
const CPSR_F: u32 = 1 << 6;
#[cfg(test)]
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
    InvalidInputTimestamp { requested: Cycle, earliest: Cycle },
    InputQueueFull,
    EmptyRom,
    RomTooLarge { size: usize, maximum: usize },
    InvalidAccessAlignment { address: u32, width: usize },
    UnmappedAddress { address: u32, width: usize },
    UnsupportedInstruction { address: u32, instruction: u32 },
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInputTimestamp {
                requested,
                earliest,
            } => write!(
                formatter,
                "input cycle {} precedes earliest allowed cycle {}",
                requested.0, earliest.0
            ),
            Self::InputQueueFull => formatter.write_str("timestamped input queue is full"),
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
    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError>;
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError>;
    fn read16(&mut self, address: u32, access: Access) -> Result<u16, CoreError>;
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError>;

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError>;
    fn write32(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError>;

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
        if instruction & 0x0E00_0000 == 0x0A00_0000 {
            if instruction & (1 << 24) != 0 {
                self.registers[14] = address.wrapping_add(4);
            }
            let offset = ((instruction & 0x00FF_FFFF) << 8) as i32 >> 6;
            let target = address.wrapping_add(8).wrapping_add(offset as u32);
            self.registers[15] = target;
            self.refill(bus)?;
        } else if self.execute_multiply(instruction, bus) {
            // Multiply timing adds internal cycles to the already charged fetch.
        } else if self.execute_data_processing(address, instruction) {
            // The immediate shifts used here require no additional idle cycle.
        } else if self.execute_word_transfer(address, instruction, bus)?
            || self.execute_block_transfer(address, instruction, bus)?
            || self.execute_halfword(address, instruction, bus)?
        {
            // A data access breaks fetch sequentiality, except when an LDM
            // return already refilled the pipeline at its new instruction PC.
            let loaded_pc = instruction & 0x0E10_8000 == 0x0810_8000;
            self.next_fetch_is_sequential = loaded_pc;
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

    /// Executes immediate and immediate-shifted ARM ALU operations. Arithmetic
    /// uses one wide addition rule for carry and signed overflow; logical
    /// operations preserve V and take C from the barrel shifter. Register-
    /// controlled shifts and PC writes remain outside the current guest scope.
    fn execute_data_processing(&mut self, address: u32, instruction: u32) -> bool {
        if instruction & 0x0C00_0000 != 0
            || (instruction & (1 << 25) == 0 && instruction & (1 << 4) != 0)
        {
            return false;
        }
        let opcode = (instruction >> 21) & 15;
        let set_flags = instruction & (1 << 20) != 0;
        if matches!(opcode, 8..=11) && !set_flags {
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
        let (operand, shifter_carry) = if instruction & (1 << 25) != 0 {
            let rotate = ((instruction >> 8) & 15) * 2;
            let value = (instruction & 255).rotate_right(rotate);
            (
                value,
                if rotate == 0 {
                    self.cpsr & CPSR_C != 0
                } else {
                    value >> 31 != 0
                },
            )
        } else {
            let source = (instruction & 15) as usize;
            let value = if source == 15 {
                address.wrapping_add(8)
            } else {
                self.registers[source]
            };
            self.shift_immediate(value, (instruction >> 5) & 3, (instruction >> 7) & 31)
        };
        let carry_in = u32::from(self.cpsr & CPSR_C != 0);
        let arithmetic = match opcode {
            2 | 10 => Some((lhs, !operand, 1)),
            3 => Some((operand, !lhs, 1)),
            4 | 11 => Some((lhs, operand, 0)),
            5 => Some((lhs, operand, carry_in)),
            6 => Some((lhs, !operand, carry_in)),
            7 => Some((operand, !lhs, carry_in)),
            _ => None,
        };
        let (result, carry, overflow) = if let Some((a, b, carry)) = arithmetic {
            // Complementing the subtrahend makes carry mean "no borrow" for
            // subtraction, including SBC/RSC's inverted carry-in borrow.
            let wide = u64::from(a) + u64::from(b) + u64::from(carry);
            let result = wide as u32;
            (
                result,
                wide > u64::from(u32::MAX),
                (!(a ^ b) & (a ^ result)) & 0x8000_0000 != 0,
            )
        } else {
            let result = match opcode {
                0 | 8 => lhs & operand,
                1 | 9 => lhs ^ operand,
                12 => lhs | operand,
                13 => operand,
                14 => lhs & !operand,
                15 => !operand,
                _ => unreachable!("all arithmetic opcodes handled above"),
            };
            (result, shifter_carry, false)
        };
        if set_flags {
            self.cpsr &= !(CPSR_N | CPSR_Z);
            if result & 0x8000_0000 != 0 {
                self.cpsr |= CPSR_N;
            }
            if result == 0 {
                self.cpsr |= CPSR_Z;
            }
            if arithmetic.is_some() {
                self.cpsr &= !(CPSR_C | CPSR_V);
                if carry {
                    self.cpsr |= CPSR_C;
                }
                if overflow {
                    self.cpsr |= CPSR_V;
                }
            } else {
                self.cpsr = (self.cpsr & !CPSR_C) | if shifter_carry { CPSR_C } else { 0 };
            }
        }
        if !matches!(opcode, 8..=11) {
            self.registers[destination] = result;
        }
        true
    }

    /// Decodes ARM immediate shifts, including the zero encodings for shifts
    /// by 32 and RRX. The carry result is used only when the ALU updates flags.
    fn shift_immediate(&self, value: u32, kind: u32, amount: u32) -> (u32, bool) {
        let carry = self.cpsr & CPSR_C != 0;
        match (kind, amount) {
            (0, 0) => (value, carry),
            (0, n) => (value << n, value & (1 << (32 - n)) != 0),
            (1, 0) => (0, value >> 31 != 0),
            (1, n) => (value >> n, value & (1 << (n - 1)) != 0),
            (2, 0) => (((value as i32) >> 31) as u32, value >> 31 != 0),
            (2, n) => (((value as i32) >> n) as u32, value & (1 << (n - 1)) != 0),
            (3, 0) => ((u32::from(carry) << 31) | (value >> 1), value & 1 != 0),
            (3, n) => (value.rotate_right(n), value & (1 << (n - 1)) != 0),
            _ => unreachable!("shift encoding contains two bits"),
        }
    }

    /// MUL/MLA supply the text routine's row offset through ordinary CPU
    /// arithmetic. Timing follows ARM7 early termination of the multiplier.
    fn execute_multiply<B: CpuBus>(&mut self, instruction: u32, bus: &mut B) -> bool {
        if instruction & 0x0FC0_00F0 != 0x0000_0090 {
            return false;
        }
        let rd = ((instruction >> 16) & 15) as usize;
        let rn = ((instruction >> 12) & 15) as usize;
        let rs = ((instruction >> 8) & 15) as usize;
        let rm = (instruction & 15) as usize;
        let accumulate = instruction & (1 << 21) != 0;
        if rd == 15 || rs == 15 || rm == 15 || (accumulate && rn == 15) {
            return false;
        }
        let multiplier = self.registers[rs];
        let mut result = self.registers[rm].wrapping_mul(multiplier);
        if accumulate {
            result = result.wrapping_add(self.registers[rn]);
        }
        self.registers[rd] = result;
        if instruction & (1 << 20) != 0 {
            self.cpsr = (self.cpsr & !(CPSR_N | CPSR_Z))
                | if result >> 31 != 0 { CPSR_N } else { 0 }
                | if result == 0 { CPSR_Z } else { 0 };
        }
        let iterations = [8_u32, 16, 24]
            .into_iter()
            .find(|bits| {
                let upper = (multiplier as i32) >> bits;
                upper == 0 || upper == -1
            })
            .map_or(4, |bits| bits / 8);
        bus.idle(u64::from(iterations) + u64::from(accumulate));
        true
    }

    /// Handles immediate-offset byte and word transfers through the timed bus.
    /// Word loads rotate unaligned data; stores discard the low address bits.
    /// Shifted register offsets and privileged transfers remain deferred.
    fn execute_word_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E00_0000 != 0x0400_0000 {
            return Ok(false);
        }
        let byte = instruction & (1 << 22) != 0;
        let rn = ((instruction >> 16) & 15) as usize;
        let rd = ((instruction >> 12) & 15) as usize;
        let pre = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0 || !pre;
        let load = instruction & (1 << 20) != 0;
        if rd == 15
            || (writeback && (rn == 15 || (load && rn == rd)))
            || (!pre && instruction & (1 << 21) != 0)
        {
            return Ok(false);
        }
        let base = if rn == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[rn]
        };
        let offset = instruction & 0xfff;
        let updated = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let target = if pre { updated } else { base };
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        if load {
            self.registers[rd] = if byte {
                u32::from(bus.read8(target, access)?)
            } else {
                bus.read32(target & !3, access)?
                    .rotate_right((target & 3) * 8)
            };
            bus.idle(1);
        } else {
            if byte {
                bus.write8(target, self.registers[rd] as u8, access)?;
            } else {
                bus.write32(target & !3, self.registers[rd], access)?;
            }
        }
        if writeback {
            self.registers[rn] = updated;
        }
        Ok(true)
    }

    /// Transfers ascending register numbers to ascending addresses for ARM
    /// stack and glyph-data operations. A loaded PC refills the ARM pipeline;
    /// exception-return/user-bank and unpredictable forms are deferred.
    fn execute_block_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E40_0000 != 0x0800_0000 {
            return Ok(false);
        }
        let rn = ((instruction >> 16) & 15) as usize;
        let list = instruction & 0xffff;
        let load = instruction & (1 << 20) != 0;
        let writeback = instruction & (1 << 21) != 0;
        if rn == 15 || list == 0 || (writeback && list & (1 << rn) != 0) {
            return Ok(false);
        }
        let bytes = list.count_ones() * 4;
        let base = self.registers[rn];
        let up = instruction & (1 << 23) != 0;
        let pre = instruction & (1 << 24) != 0;
        let updated = if up {
            base.wrapping_add(bytes)
        } else {
            base.wrapping_sub(bytes)
        };
        let mut target = if up {
            base.wrapping_add(if pre { 4 } else { 0 })
        } else {
            updated.wrapping_add(if pre { 0 } else { 4 })
        };
        let mut sequential = false;
        for register in 0..16 {
            if list & (1 << register) == 0 {
                continue;
            }
            let access = Access {
                kind: AccessKind::Data,
                sequential,
            };
            if load {
                self.registers[register] = bus.read32(target, access)?;
            } else {
                let value = if register == 15 {
                    address.wrapping_add(12)
                } else {
                    self.registers[register]
                };
                bus.write32(target, value, access)?;
            }
            target = target.wrapping_add(4);
            sequential = true;
        }
        if writeback {
            self.registers[rn] = updated;
        }
        if load {
            bus.idle(1);
            if list & (1 << 15) != 0 {
                self.registers[15] &= !3;
                self.refill(bus)?;
            }
        }
        Ok(true)
    }

    /// Routes immediate/register-offset LDRH/STRH through the timed bus.
    /// Register offsets are unshifted, as required by ARM halfword transfers.
    fn execute_halfword<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E00_00F0 != 0x0000_00B0 {
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
        let offset = if instruction & (1 << 22) != 0 {
            ((instruction >> 4) & 0xF0) | (instruction & 15)
        } else {
            if instruction & 0xF00 != 0 || instruction & 15 == 15 {
                return Ok(false);
            }
            self.registers[(instruction & 15) as usize]
        };
        let updated = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        let target = if pre { updated } else { base };
        if instruction & (1 << 20) != 0 {
            if (!pre || writeback) && base_register == source {
                return Ok(false);
            }
            self.registers[source] = bus.read16(target, access)? as u32;
            bus.idle(1);
        } else {
            bus.write16(target, self.registers[source] as u16, access)?;
        }
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
    buttons: ButtonState,
    inputs: VecDeque<InputEvent>,
    rom: Vec<u8>,
    ewram: Vec<u8>,
    iwram: Vec<u8>,
    io: Vec<u8>,
    /// Shared BG/OBJ color RAM; mode 4 indexes the first 256 entries.
    palette: Vec<u8>,
    vram: Vec<u8>,
    display: Display,
    cycles: u64,
}

impl System {
    fn new() -> Self {
        let mut system = Self {
            buttons: ButtonState::default(),
            inputs: VecDeque::new(),
            rom: Vec::new(),
            ewram: vec![0; 256 * 1024],
            iwram: vec![0; 32 * 1024],
            io: vec![0; IO_BYTES],
            palette: vec![0; PALETTE_BYTES],
            vram: vec![0; VRAM_BYTES],
            display: Display::new(),
            cycles: 0,
        };
        system.refresh_status();
        system
    }

    /// Refreshes read-only register bits from hardware time, even during blank lines.
    /// Scanline rendering remains separate; IRQ delivery is a later slice.
    fn refresh_status(&mut self) {
        let line = (self.cycles / CYCLES_PER_SCANLINE % SCANLINES_PER_FRAME) as u16;
        let control = u16::from_le_bytes([self.io[4], self.io[5]]) & 0xff38;
        let flags = u16::from((160..227).contains(&line))
            | (u16::from(self.cycles % CYCLES_PER_SCANLINE >= HBLANK_FLAG_CYCLE) << 1)
            | (u16::from(line == control >> 8) << 2);
        self.io[4..6].copy_from_slice(&(control | flags).to_le_bytes());
        self.io[6..8].copy_from_slice(&line.to_le_bytes());
        self.io[0x130..0x132].copy_from_slice(&(!self.buttons.0 & 0x03ff).to_le_bytes());
    }

    fn read16_impl(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }
        self.charge(address, 2, access);
        let bytes = self.read_bytes(address, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
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

        self.write_halfword(address, value)
    }

    /// Applies a bus beat after its access time has synchronized scanout. Word
    /// transfers reuse this path without charging the hardware clock twice.
    fn write_halfword(&mut self, address: u32, value: u16) -> Result<(), CoreError> {
        let bytes = value.to_le_bytes();

        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), 2) {
            self.ewram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), 2) {
            self.iwram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = range_for(address, IO_START, self.io.len(), 2) {
            let offset = range.start;
            if matches!(offset, 6 | 0x130) {
                return Ok(());
            }
            self.io[range].copy_from_slice(&bytes);
            if offset == 4 {
                self.refresh_status();
            }

            if offset == 0 {
                self.display.control = value;
            }

            return Ok(());
        }

        if let Some(range) = palette_range(address, 2) {
            self.palette[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = range_for(address, VRAM_START, self.vram.len(), 2) {
            self.vram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        Err(CoreError::UnmappedAddress { address, width: 2 })
    }

    fn write32_impl(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError> {
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);
        self.write_halfword(address, value as u16)?;
        self.write_halfword(address.wrapping_add(2), (value >> 16) as u16)
    }

    fn read_bytes(&self, address: u32, width: usize) -> Result<&[u8], CoreError> {
        if let Some(range) = range_for(address, ROM_START, self.rom.len(), width) {
            return Ok(&self.rom[range]);
        }
        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), width) {
            return Ok(&self.ewram[range]);
        }
        if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), width) {
            return Ok(&self.iwram[range]);
        }
        if let Some(range) = range_for(address, IO_START, self.io.len(), width) {
            return Ok(&self.io[range]);
        }
        if let Some(range) = palette_range(address, width) {
            return Ok(&self.palette[range]);
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
            } else if ram_range(address, EWRAM_START, 256 * 1024, 1).is_some() {
                3 * beats
            } else if ram_range(address, IWRAM_START, 32 * 1024, 1).is_some()
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
        while self
            .inputs
            .front()
            .is_some_and(|event| event.cycle.0 <= target)
        {
            let event = self.inputs.pop_front().expect("front was present");
            self.buttons.set(event.button, event.pressed);
        }
        self.display
            .synchronize_to(target, &self.vram, &self.palette);
        self.cycles = target;
        self.refresh_status();
    }
}

impl CpuBus for System {
    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError> {
        self.charge(address, 1, access);
        Ok(self.read_bytes(address, 1)?[0])
    }

    /// Video memory has a 16-bit write bus: palette and BG bytes are duplicated,
    /// while OBJ VRAM ignores byte stores. RAM bytes retain ordinary semantics.
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError> {
        self.charge(address, 1, access);
        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), 1) {
            self.ewram[range.start] = value;
        } else if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), 1) {
            self.iwram[range.start] = value;
        } else if palette_range(address, 1).is_some() {
            self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
        } else if let Some(range) = range_for(address, VRAM_START, self.vram.len(), 1) {
            let object_start = if self.display.control & 7 >= 3 {
                0x14000
            } else {
                0x10000
            };
            if range.start < object_start {
                self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
            }
        } else {
            return Err(CoreError::UnmappedAddress { address, width: 1 });
        }
        Ok(())
    }

    fn read16(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        self.read16_impl(address, access)
    }
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        self.read32_impl(address, access)
    }

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError> {
        self.write16_impl(address, value, access)
    }

    fn write32(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError> {
        self.write32_impl(address, value, access)
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
    fn synchronize_to(&mut self, target: u64, vram: &[u8], palette: &[u8]) {
        while self.next_event <= target {
            if self.line < SCREEN_HEIGHT {
                let mode = self.control & 7;
                let enabled = matches!(mode, 3 | 4) && self.control & (1 << 10) != 0;
                // Mode 4 pages are separated by 40 KiB, although only 38,400
                // bytes in each page are visible. Mode 3 ignores page selection.
                let page = if self.control & (1 << 4) != 0 {
                    0xa000
                } else {
                    0
                };
                let forced_blank = self.control & (1 << 7) != 0;
                let start = self.line * SCREEN_WIDTH;
                for index in start..start + SCREEN_WIDTH {
                    self.drawing[index] = if forced_blank {
                        0x7FFF
                    } else if enabled && mode == 3 {
                        u16::from_le_bytes([vram[index * 2], vram[index * 2 + 1]]) & 0x7FFF
                    } else if enabled {
                        let color = usize::from(vram[page + index]) * 2;
                        // Index zero resolves to the backdrop color in BG palette
                        // entry zero; it must not become a hard-coded black pixel.
                        u16::from_le_bytes([palette[color], palette[color + 1]]) & 0x7FFF
                    } else {
                        u16::from_le_bytes([palette[0], palette[1]]) & 0x7FFF
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
    /// Applies a live host transition at the current instruction boundary.
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        self.system.buttons.set(button, pressed);
        self.system.refresh_status();
    }

    /// Reports the logical state actually visible to the guest keypad.
    pub fn button_pressed(&self, button: Button) -> bool {
        self.system.buttons.pressed(button)
    }
    /// Schedules a button transition on an absolute cycle. Events must be ordered;
    /// equal timestamps retain submission order, and the queue has a fixed bound.
    pub fn set_button_at(
        &mut self,
        cycle: Cycle,
        button: Button,
        pressed: bool,
    ) -> Result<(), CoreError> {
        let earliest = self
            .system
            .inputs
            .back()
            .map_or(self.cycles(), |event| event.cycle);
        if cycle < earliest {
            return Err(CoreError::InvalidInputTimestamp {
                requested: cycle,
                earliest,
            });
        }
        if self.system.inputs.len() >= 1024 {
            return Err(CoreError::InputQueueFull);
        }
        self.system.inputs.push_back(InputEvent {
            cycle,
            button,
            pressed,
        });
        self.system.advance_time(0);
        Ok(())
    }

    /// Cancels queued transitions and releases hardware keys at the current cycle.
    /// Used by pause/focus/reset boundaries so an old press cannot reappear later.
    pub fn release_all_buttons(&mut self) {
        self.system.inputs.clear();
        self.system.buttons.release_all();
        self.system.refresh_status();
    }
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

/// Palette RAM repeats throughout its 16 MiB bus region. Access widths and
/// alignment are checked by the caller before applying the physical RAM mask.
fn palette_range(address: u32, width: usize) -> Option<Range<usize>> {
    if address >> 24 != PALETTE_START >> 24 {
        return None;
    }
    let start = address as usize & (PALETTE_BYTES - 1);
    (start + width <= PALETTE_BYTES).then_some(start..start + width)
}

/// Work RAM repeats throughout its region; decode before accessing backing storage.
fn ram_range(address: u32, start: u32, length: usize, width: usize) -> Option<Range<usize>> {
    if address >> 24 != start >> 24 {
        return None;
    }
    let offset = address as usize & (length - 1);
    (offset.checked_add(width)? <= length).then_some(offset..offset + width)
}

fn range_for(address: u32, start: u32, length: usize, width: usize) -> Option<Range<usize>> {
    let offset = address.checked_sub(start)? as usize;
    let end = offset.checked_add(width)?;
    (end <= length).then_some(offset..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Catches a guest seeing pressed keys at reset or writable hardware status.
    // Existing tests exercise VRAM and CPU startup, not hardware input reads.
    #[test]
    fn keypad_is_active_low_and_hardware_status_is_read_only() {
        let mut machine = Machine::new();
        assert_eq!(machine.inspect16(IO_START + 0x130).unwrap(), 0x03ff);
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine
            .system
            .write16_impl(IO_START + 0x130, 0, access)
            .unwrap();
        machine
            .system
            .write16_impl(IO_START + 6, 99, access)
            .unwrap();
        assert_eq!(machine.inspect16(IO_START + 0x130).unwrap(), 0x03ff);
        assert_eq!(machine.inspect16(IO_START + 6).unwrap(), 0);
    }

    // Catches VBlank polling loops hanging, HBlank flags staying high, and
    // VCOUNT comparisons using frontend redraws instead of the hardware clock.
    #[test]
    fn display_status_tracks_line_blank_and_compare_boundaries() {
        let mut machine = Machine::new();
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine
            .system
            .write16_impl(IO_START + 4, (160 << 8) | 0x38, access)
            .unwrap();
        for (cycle, line, flags) in [
            (960, 0, 0),
            (1007, 0, 0),
            (1008, 0, 2),
            (1232, 1, 0),
            (160 * 1232, 160, 5),
            (227 * 1232, 227, 0),
            (CYCLES_PER_FRAME, 0, 0),
        ] {
            machine.system.advance_time(cycle - machine.cycles().0);
            assert_eq!(machine.inspect16(IO_START + 6).unwrap(), line);
            assert_eq!(
                machine.inspect16(IO_START + 4).unwrap(),
                (160 << 8) | 0x38 | flags
            );
        }
    }

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
