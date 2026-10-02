#![forbid(unsafe_code)]

mod backup;
mod eeprom;
pub use eeprom::{EEPROM8K_BYTES, EEPROM512_BYTES};
mod flash;
mod sram;
pub use backup::{BackupDetection, BackupSelection, BackupType, detect_backup};
pub use flash::{FLASH64_BYTES, FLASH128_BYTES};
pub use sram::{SRAM_BYTES, SaveImage, SaveStatus};

mod audio;
mod noise;
mod pulse;
mod wave;
pub use audio::PCM_RATE;

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
    pub fn set(&mut self, button: Button, pressed: bool) -> bool {
        let mask = 1 << button as u8;
        let before = self.0;

        if pressed {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }

        self.0 != before
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

/// Original division firmware, never a replacement for the retail BIOS.
pub const TEST_FIRMWARE: &[u8] = include_bytes!("../../../roms/test-firmware/division.bin");

/// Exact length of the supplied ARM7TDMI BIOS image.
pub const BIOS_SIZE: usize = 0x4000;

const ROM_START: u32 = 0x0800_0000;
const MAX_ROM_BYTES: usize = 32 * 1024 * 1024;
const EWRAM_START: u32 = 0x0200_0000;
const IWRAM_START: u32 = 0x0300_0000;
const IO_START: u32 = 0x0400_0000;
const IO_BYTES: usize = 0x400;
/// Undocumented write-only location touched by the retail BIOS during
/// RegisterRamReset. The written value has no modeled observable effect.
const BIOS_UNDOCUMENTED_IO_410: u32 = IO_START + 0x410;
const PALETTE_START: u32 = 0x0500_0000;
const PALETTE_BYTES: usize = 1024;
const VRAM_START: u32 = 0x0600_0000;
const VRAM_BYTES: usize = 96 * 1024;
const OAM_START: u32 = 0x0700_0000;
const OAM_BYTES: usize = 1024;
// The scanline renderer samples at 960 cycles, but the DISPSTAT HBlank flag
// follows the 1008-cycle mGBA timing model. IRQ edge timing is a later slice.
const HBLANK_FLAG_CYCLE: u64 = 1_008;

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
    InvalidInputTimestamp {
        requested: Cycle,
        earliest: Cycle,
    },
    InputQueueFull,
    EmptyRom,
    MissingBios,
    InvalidBiosSize {
        size: usize,
    },
    RomTooLarge {
        size: usize,
        maximum: usize,
    },
    InvalidAccessAlignment {
        address: u32,
        width: usize,
    },
    UnmappedAddress {
        address: u32,
        width: usize,
    },
    UnsupportedInstruction {
        address: u32,
        instruction: u32,
    },
    /// Halfword load/store execution received an unsupported internal selector.
    InvalidTransferKind {
        kind: u32,
    },
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
            Self::MissingBios => {
                formatter.write_str("load a supplied 16 KiB BIOS before booting a cartridge")
            }
            Self::InvalidBiosSize { size } => {
                write!(formatter, "BIOS is {size} bytes; expected {BIOS_SIZE}")
            }
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
                "unsupported CPU instruction {instruction:#010x} at {address:#010x}"
            ),
            Self::InvalidTransferKind { kind } => {
                write!(formatter, "invalid internal halfword transfer kind {kind}")
            }
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
    /// Supplies the executing instruction address independently of pipeline lookahead.
    fn set_execution_address(&mut self, address: u32);
    /// Publishes instruction bus context before any operand accesses overwrite data latches.
    fn set_cpu_open_bus(&mut self, pipeline: [u32; 2], address: u32, thumb: bool);
    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError>;
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError>;
    fn read16(&mut self, address: u32, access: Access) -> Result<u16, CoreError>;
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError>;

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError>;
    fn write32(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError>;

    fn idle(&mut self, cycles: u64);
    /// Signals a pipeline redirect; the bus discards cartridge prefetch state.
    fn restart_fetch(&mut self);
}

struct Cpu {
    registers: [u32; 16],
    pipeline: [u32; 2],
    pipeline_valid: bool,
    next_fetch_is_sequential: bool,
    cpsr: u32,
    /// User/system and FIQ banks hold r8-r14 across mode changes.
    user_bank: [u32; 7],
    fiq_bank: [u32; 7],
    /// IRQ, supervisor, abort, and undefined modes bank SP/LR and SPSR.
    exception_banks: [[u32; 2]; 4],
    saved_status: [u32; 5],
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
            user_bank: [0; 7],
            fiq_bank: [0; 7],
            exception_banks: [[0; 2]; 4],
            saved_status: [0; 5],
        }
    }

    /// Returns the privileged bank index; user and system share one bank.
    fn status_bank(mode: u32) -> Option<usize> {
        match mode & 31 {
            0x11 => Some(0),
            0x12 => Some(1),
            0x13 => Some(2),
            0x17 => Some(3),
            0x1b => Some(4),
            _ => None,
        }
    }

    /// Saves outgoing banked registers before exposing the incoming mode.
    fn set_status(&mut self, value: u32) {
        let old = Self::status_bank(self.cpsr);
        let new = Self::status_bank(value);
        if old != new {
            if old == Some(0) {
                self.fiq_bank.copy_from_slice(&self.registers[8..15]);
                self.registers[8..15].copy_from_slice(&self.user_bank);
            } else {
                self.user_bank[..5].copy_from_slice(&self.registers[8..13]);
                if let Some(bank) = old {
                    self.exception_banks[bank - 1].copy_from_slice(&self.registers[13..15]);
                } else {
                    self.user_bank[5..].copy_from_slice(&self.registers[13..15]);
                }
            }
            if new == Some(0) {
                self.registers[8..15].copy_from_slice(&self.fiq_bank);
            } else if let Some(bank) = new {
                self.registers[13..15].copy_from_slice(&self.exception_banks[bank - 1]);
            } else {
                self.registers[13..15].copy_from_slice(&self.user_bank[5..]);
            }
        }
        self.cpsr = value;
    }

    /// MRS/MSR apply byte-field masks and privilege rules before ALU decoding.
    fn execute_status_transfer(&mut self, instruction: u32) -> bool {
        let saved = instruction & (1 << 22) != 0;
        if instruction & 0x0fbf_0fff == 0x010f_0000 {
            let rd = ((instruction >> 12) & 15) as usize;
            self.registers[rd] = if saved {
                Self::status_bank(self.cpsr).map_or(self.cpsr, |b| self.saved_status[b])
            } else {
                self.cpsr
            };
            return true;
        }
        if instruction & 0x0db0_f000 != 0x0120_f000 {
            return false;
        }
        let value = if instruction & (1 << 25) != 0 {
            (instruction & 255).rotate_right(((instruction >> 8) & 15) * 2)
        } else {
            self.registers[(instruction & 15) as usize]
        };
        let mut mask = 0;
        for byte in 0..4 {
            if instruction & (1 << (16 + byte)) != 0 {
                mask |= 255 << (byte * 8);
            }
        }
        if saved {
            if let Some(bank) = Self::status_bank(self.cpsr) {
                self.saved_status[bank] = (self.saved_status[bank] & !mask) | (value & mask);
            }
        } else {
            if self.cpsr & 31 == 0x10 {
                mask &= 0xff00_0000;
            }
            // ARMv4 MSR cannot switch instruction sets; exception return can.
            mask &= !CPSR_T;
            self.set_status((self.cpsr & !mask) | (value & mask));
        }
        true
    }

    /// SWI enters supervisor ARM state, banks LR/SP, and maps the return
    /// state through SPSR. Service execution remains ordinary guest code.
    fn software_interrupt<B: CpuBus>(
        &mut self,
        bus: &mut B,
        return_address: u32,
    ) -> Result<(), CoreError> {
        let status = self.cpsr;
        self.set_status((status & !(31 | CPSR_T)) | 0x13 | CPSR_I);
        self.saved_status[2] = status;
        self.registers[14] = return_address;
        self.branch(bus, 8, false)
    }

    /// Fills the two-stage instruction pipeline after reset or a taken branch.
    fn refill<B: CpuBus>(&mut self, bus: &mut B) -> Result<(), CoreError> {
        let pc = self.registers[15];
        bus.set_execution_address(pc);
        bus.restart_fetch();
        self.pipeline[0] = self.fetch(
            bus,
            pc,
            Access {
                kind: AccessKind::Fetch,
                sequential: false,
            },
        )?;
        self.pipeline[1] = self.fetch(
            bus,
            pc.wrapping_add(self.instruction_width()),
            Access {
                kind: AccessKind::Fetch,
                sequential: true,
            },
        )?;
        self.pipeline_valid = true;
        self.next_fetch_is_sequential = true;
        Ok(())
    }

    /// Instruction width follows CPSR.T; register 15 stores the next executed
    /// address, while individual operands expose the architectural pipeline PC.
    fn instruction_width(&self) -> u32 {
        if self.cpsr & CPSR_T != 0 { 2 } else { 4 }
    }

    fn fetch<B: CpuBus>(
        &self,
        bus: &mut B,
        address: u32,
        access: Access,
    ) -> Result<u32, CoreError> {
        if self.cpsr & CPSR_T != 0 {
            bus.read16(address, access).map(u32::from)
        } else {
            bus.read32(address, access)
        }
    }

    /// BX alone selects state from bit zero. Ordinary PC writes retain state;
    /// every taken branch discards the old pipeline before fetching its target.
    fn branch<B: CpuBus>(
        &mut self,
        bus: &mut B,
        target: u32,
        exchange: bool,
    ) -> Result<(), CoreError> {
        if exchange {
            self.cpsr = (self.cpsr & !CPSR_T) | if target & 1 != 0 { CPSR_T } else { 0 };
        }
        self.registers[15] = target & !(self.instruction_width() - 1);
        self.refill(bus)
    }

    /// Executes from the pipeline and accounts for fetch, data, and internal cycles once.
    fn step<B: CpuBus>(&mut self, bus: &mut B) -> Result<StepOutcome, CoreError> {
        if !self.pipeline_valid {
            self.refill(bus)?;
        }
        let address = self.registers[15];
        bus.set_execution_address(address);
        let instruction = self.pipeline[0];
        let width = self.instruction_width();
        let next = self.fetch(
            bus,
            address.wrapping_add(width * 2),
            Access {
                kind: AccessKind::Fetch,
                sequential: self.next_fetch_is_sequential,
            },
        )?;
        self.pipeline = [self.pipeline[1], next];
        bus.set_cpu_open_bus(self.pipeline, address, self.cpsr & CPSR_T != 0);
        self.registers[15] = address.wrapping_add(width);
        self.next_fetch_is_sequential = true;
        if self.cpsr & CPSR_T != 0 {
            self.execute_thumb(address, instruction as u16, bus)?;
            return Ok(StepOutcome { address });
        }
        if !self.condition_passes(instruction >> 28) {
            return Ok(StepOutcome { address });
        }
        if instruction & 0x0f00_0000 == 0x0f00_0000 {
            self.software_interrupt(bus, address.wrapping_add(4))?;
        } else if instruction & 0x0fff_fff0 == 0x012f_ff10 {
            let source = (instruction & 15) as usize;
            let target = if source == 15 {
                address.wrapping_add(8)
            } else {
                self.registers[source]
            };
            self.branch(bus, target, true)?;
        } else if instruction & 0x0E00_0000 == 0x0A00_0000 {
            if instruction & (1 << 24) != 0 {
                self.registers[14] = address.wrapping_add(4);
            }
            let offset = ((instruction & 0x00FF_FFFF) << 8) as i32 >> 6;
            let target = address.wrapping_add(8).wrapping_add(offset as u32);
            self.registers[15] = target;
            self.refill(bus)?;
        } else if self.execute_status_transfer(instruction) {
            // Status transfers update the selected register bank atomically.
        } else if self.execute_multiply(instruction, bus) {
            // Multiply timing adds internal cycles to the already charged fetch.
        } else if self.execute_data_processing(address, instruction) {
            if instruction & (1 << 25) == 0 && instruction & (1 << 4) != 0 {
                bus.idle(1);
            }
            if (instruction >> 12) & 15 == 15 && !matches!((instruction >> 21) & 15, 8..=11) {
                self.branch(bus, self.registers[15], false)?;
            }
        } else if self.execute_swap(instruction, bus)?
            || self.execute_word_transfer(address, instruction, bus)?
            || self.execute_block_transfer(address, instruction, bus)?
            || self.execute_halfword(address, instruction, bus)?
        {
            // A data access breaks fetch sequentiality, except when an LDM
            // return already refilled the pipeline at its new instruction PC.
            let loaded_pc = instruction & (1 << 20) != 0
                && ((instruction & 0x0e00_0000 == 0x0800_0000 && instruction & 0x8000 != 0)
                    || (instruction & 0x0c00_0000 == 0x0400_0000
                        && (instruction >> 12) & 15 == 15));
            self.next_fetch_is_sequential = loaded_pc;
        } else {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }
        Ok(StepOutcome { address })
    }

    /// Decodes ARMv4T Thumb forms into the shared ALU, transfer, and pipeline
    /// paths. Unsupported encodings remain explicit execution errors.
    fn execute_thumb<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u16,
        bus: &mut B,
    ) -> Result<(), CoreError> {
        let op = u32::from(instruction);
        let rd = (op & 7) as usize;
        let rn = ((op >> 3) & 7) as usize;
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        if op & 0xe000 == 0 && op & 0x1800 != 0x1800 {
            // Immediate shifts share ARM's zero-shift encodings and carry rules.
            let arm = (13 << 21)
                | (1 << 20)
                | ((rd as u32) << 12)
                | (((op >> 11) & 3) << 5)
                | (((op >> 6) & 31) << 7)
                | rn as u32;
            self.execute_data_processing(address, arm);
        } else if op & 0xf800 == 0x1800 {
            let operand = (op >> 6) & 7;
            let arm = (if op & 0x0400 != 0 { 1 << 25 } else { 0 })
                | ((if op & 0x0200 != 0 { 2 } else { 4 }) << 21)
                | (1 << 20)
                | ((rn as u32) << 16)
                | ((rd as u32) << 12)
                | operand;
            self.execute_data_processing(address, arm);
        } else if op & 0xe000 == 0x2000 {
            let destination = (op >> 8) & 7;
            let opcode = [13, 10, 4, 2][((op >> 11) & 3) as usize];
            let arm = (1 << 25)
                | (opcode << 21)
                | (1 << 20)
                | (destination << 16)
                | (destination << 12)
                | (op & 255);
            self.execute_data_processing(address, arm);
        } else if op & 0xfc00 == 0x4000 {
            let operation = (op >> 6) & 15;
            if matches!(operation, 2..=4 | 7) {
                let kind = if operation == 7 { 3 } else { operation - 2 };
                let arm = (13 << 21)
                    | (1 << 20)
                    | ((rd as u32) << 12)
                    | ((rn as u32) << 8)
                    | (kind << 5)
                    | (1 << 4)
                    | rd as u32;
                self.execute_data_processing(address, arm);
                bus.idle(1);
            } else if operation == 13 {
                let arm = (1 << 20) | ((rd as u32) << 16) | ((rn as u32) << 8) | 0x90 | rd as u32;
                self.execute_multiply(arm, bus);
            } else {
                let opcode =
                    [0, 1, 0, 0, 0, 5, 6, 0, 8, 3, 10, 11, 12, 0, 14, 15][operation as usize];
                let arm = (opcode << 21)
                    | (1 << 20)
                    | rn as u32
                    | ((rd as u32) << 16)
                    | ((rd as u32) << 12);
                let arm = if operation == 9 {
                    (arm & !0x000f_000f) | (1 << 25) | ((rn as u32) << 16)
                } else {
                    arm
                };
                self.execute_data_processing(address, arm);
            }
        } else if op & 0xfc00 == 0x4400 {
            let destination = ((op & 7) | ((op >> 4) & 8)) as usize;
            let source = ((op >> 3) & 15) as usize;
            let operand = if source == 15 {
                address.wrapping_add(4)
            } else {
                self.registers[source]
            };
            match (op >> 8) & 3 {
                0 | 2 => {
                    let lhs = if destination == 15 {
                        address.wrapping_add(4)
                    } else {
                        self.registers[destination]
                    };
                    let value = if op & 0x0200 == 0 {
                        lhs.wrapping_add(operand)
                    } else {
                        operand
                    };
                    if destination == 15 {
                        self.branch(bus, value, false)?;
                    } else {
                        self.registers[destination] = value;
                    }
                }
                1 => {
                    let arm = (10 << 21) | (1 << 20) | ((destination as u32) << 16) | source as u32;
                    self.execute_data_processing(address.wrapping_sub(4), arm);
                }
                3 if op & 0x0087 == 0 => self.branch(bus, operand, true)?,
                _ => {
                    return Err(CoreError::UnsupportedInstruction {
                        address,
                        instruction: op,
                    });
                }
            }
        } else if op & 0xf800 == 0x4800 {
            let target = (address.wrapping_add(4) & !3).wrapping_add((op & 255) * 4);
            self.registers[((op >> 8) & 7) as usize] = bus.read32(target, access)?;
            bus.idle(1);
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0x5000 {
            let target = self.registers[rn].wrapping_add(self.registers[((op >> 6) & 7) as usize]);
            match (op >> 9) & 7 {
                0 => bus.write32(store_address(target, 3), self.registers[rd], access)?,
                1 => bus.write16(store_address(target, 1), self.registers[rd] as u16, access)?,
                2 => bus.write8(target, self.registers[rd] as u8, access)?,
                3 => self.registers[rd] = self.load_halfword_signed(bus, target, 2, access)?,
                4 => {
                    self.registers[rd] = bus
                        .read32(target & !3, access)?
                        .rotate_right((target & 3) * 8)
                }
                5 => self.registers[rd] = self.load_halfword_signed(bus, target, 1, access)?,
                6 => self.registers[rd] = u32::from(bus.read8(target, access)?),
                7 => self.registers[rd] = self.load_halfword_signed(bus, target, 3, access)?,
                _ => {
                    return Err(CoreError::UnsupportedInstruction {
                        address,
                        instruction: op,
                    });
                }
            }
            if op & 0x0800 != 0 || op & 0x0e00 == 0x0600 {
                bus.idle(1);
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xe000 == 0x6000 || op & 0xf000 == 0x8000 {
            let halfword = op & 0xf000 == 0x8000;
            let byte = !halfword && op & 0x1000 != 0;
            let offset = ((op >> 6) & 31)
                * if halfword {
                    2
                } else if byte {
                    1
                } else {
                    4
                };
            let target = self.registers[rn].wrapping_add(offset);
            if op & 0x0800 != 0 {
                self.registers[rd] = if halfword {
                    self.load_halfword_signed(bus, target, 1, access)?
                } else if byte {
                    u32::from(bus.read8(target, access)?)
                } else {
                    bus.read32(target & !3, access)?
                        .rotate_right((target & 3) * 8)
                };
                bus.idle(1);
            } else if halfword {
                bus.write16(store_address(target, 1), self.registers[rd] as u16, access)?;
            } else if byte {
                bus.write8(target, self.registers[rd] as u8, access)?;
            } else {
                bus.write32(store_address(target, 3), self.registers[rd], access)?;
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0x9000 {
            let target = self.registers[13].wrapping_add((op & 255) * 4);
            let register = ((op >> 8) & 7) as usize;
            if op & 0x0800 != 0 {
                self.registers[register] = bus
                    .read32(target & !3, access)?
                    .rotate_right((target & 3) * 8);
                bus.idle(1);
            } else {
                bus.write32(store_address(target, 3), self.registers[register], access)?;
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0xa000 {
            let base = if op & 0x0800 == 0 {
                address.wrapping_add(4) & !3
            } else {
                self.registers[13]
            };
            self.registers[((op >> 8) & 7) as usize] = base.wrapping_add((op & 255) * 4);
        } else if op & 0xf600 == 0xb400 {
            let load = op & 0x0800 != 0;
            let list = (op & 255)
                | if op & 0x0100 != 0 {
                    1 << if load { 15 } else { 14 }
                } else {
                    0
                };
            let arm = 0x0800_0000
                | (13 << 16)
                | (1 << 21)
                | list
                | if load { (1 << 23) | (1 << 20) } else { 1 << 24 };
            self.execute_block_transfer(address, arm, bus)?;
            self.next_fetch_is_sequential = load && list & (1 << 15) != 0;
        } else if op & 0xf000 == 0xc000 {
            let list = op & 255;
            let load = op & 0x0800 != 0;
            let arm = 0x08a0_0000 | (((op >> 8) & 7) << 16) | list | if load { 1 << 20 } else { 0 };
            self.execute_block_transfer(address, arm, bus)?;
            self.next_fetch_is_sequential = load && list == 0;
        } else if op & 0xff00 == 0xb000 {
            let offset = (op & 127) * 4;
            self.registers[13] = if op & 128 == 0 {
                self.registers[13].wrapping_add(offset)
            } else {
                self.registers[13].wrapping_sub(offset)
            };
        } else if op & 0xff00 == 0xdf00 {
            self.software_interrupt(bus, address.wrapping_add(2))?;
        } else if op & 0xf000 == 0xd000 && (op >> 8) & 15 < 14 {
            if self.condition_passes((op >> 8) & 15) {
                let offset = (instruction as u8 as i8 as i32) * 2;
                self.branch(
                    bus,
                    address.wrapping_add(4).wrapping_add(offset as u32),
                    false,
                )?;
            }
        } else if op & 0xf800 == 0xe000 {
            let offset = ((op & 0x07ff) << 21) as i32 >> 20;
            self.branch(
                bus,
                address.wrapping_add(4).wrapping_add(offset as u32),
                false,
            )?;
        } else if op & 0xf800 == 0xf000 {
            // ARMv4T BL is two independently executed halfwords. The prefix
            // places the sign-extended high offset in LR; the suffix tags return.
            let offset = ((op & 0x07ff) << 21) as i32 >> 9;
            self.registers[14] = address.wrapping_add(4).wrapping_add(offset as u32);
        } else if op & 0xf800 == 0xf800 {
            let target = self.registers[14].wrapping_add((op & 0x07ff) * 2);
            self.registers[14] = address.wrapping_add(2) | 1;
            self.branch(bus, target, false)?;
        } else {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction: op,
            });
        }
        Ok(())
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

    /// Executes ARM ALU operations with immediate or register-controlled shifts.
    /// Arithmetic shares carry/overflow rules; logical operations preserve V.
    /// S-bit PC destinations restore SPSR, including legacy comparison forms;
    /// only operations writing PC redirect execution and refill the pipeline.
    fn execute_data_processing(&mut self, address: u32, instruction: u32) -> bool {
        if instruction & 0x0C00_0000 != 0
            || (instruction & (1 << 25) == 0 && instruction & 0x90 == 0x90)
        {
            return false;
        }
        let opcode = (instruction >> 21) & 15;
        let set_flags = instruction & (1 << 20) != 0;
        if matches!(opcode, 8..=11) && !set_flags {
            return false;
        }
        let destination = ((instruction >> 12) & 15) as usize;
        let source = ((instruction >> 16) & 15) as usize;
        let lhs = if source == 15 {
            address.wrapping_add(if instruction & 0x0200_0010 == 0x10 {
                12
            } else {
                8
            })
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
            if instruction & (1 << 4) != 0 {
                let value = if source == 15 {
                    address.wrapping_add(12)
                } else {
                    value
                };
                let rs = ((instruction >> 8) & 15) as usize;
                let amount = if rs == 15 {
                    address.wrapping_add(8)
                } else {
                    self.registers[rs]
                } & 255;
                self.shift_register(value, (instruction >> 5) & 3, amount)
            } else {
                self.shift_immediate(value, (instruction >> 5) & 3, (instruction >> 7) & 31)
            }
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
                _ => return false,
            };
            (result, shifter_carry, false)
        };
        if set_flags && destination == 15 {
            if let Some(bank) = Self::status_bank(self.cpsr) {
                self.set_status(self.saved_status[bank]);
            }
        } else if set_flags {
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

    /// Register shifts use an eight-bit count, preserving carry at zero and
    /// handling counts at or beyond the word width without host overshifts.
    fn shift_register(&self, value: u32, kind: u32, amount: u32) -> (u32, bool) {
        if amount == 0 {
            return (value, self.cpsr & CPSR_C != 0);
        }
        match kind & 3 {
            0 if amount < 32 => (value << amount, value & (1 << (32 - amount)) != 0),
            0 => (0, amount == 32 && value & 1 != 0),
            1 if amount < 32 => (value >> amount, value & (1 << (amount - 1)) != 0),
            1 => (0, amount == 32 && value >> 31 != 0),
            2 => (
                ((value as i32) >> amount.min(31)) as u32,
                value & (1 << (amount.min(32) - 1)) != 0,
            ),
            _ => {
                let result = value.rotate_right(amount);
                (result, result >> 31 != 0)
            }
        }
    }

    /// Decodes ARM immediate shifts, including the zero encodings for shifts
    /// by 32 and RRX. The carry result is used only when the ALU updates flags.
    fn shift_immediate(&self, value: u32, kind: u32, amount: u32) -> (u32, bool) {
        let carry = self.cpsr & CPSR_C != 0;
        match (kind & 3, amount) {
            (0, 0) => (value, carry),
            (0, n) => (value << n, value & (1 << (32 - n)) != 0),
            (1, 0) => (0, value >> 31 != 0),
            (1, n) => (value >> n, value & (1 << (n - 1)) != 0),
            (2, 0) => (((value as i32) >> 31) as u32, value >> 31 != 0),
            (2, n) => (((value as i32) >> n) as u32, value & (1 << (n - 1)) != 0),
            (_, 0) => ((u32::from(carry) << 31) | (value >> 1), value & 1 != 0),
            (_, n) => (value.rotate_right(n), value & (1 << (n - 1)) != 0),
        }
    }

    /// Executes short and signed/unsigned long multiplies, including accumulate
    /// forms. Internal timing follows ARM7 multiplier early termination.
    fn execute_multiply<B: CpuBus>(&mut self, instruction: u32, bus: &mut B) -> bool {
        if instruction & 0x0f80_00f0 == 0x0080_0090 {
            let hi = ((instruction >> 16) & 15) as usize;
            let lo = ((instruction >> 12) & 15) as usize;
            let rs = ((instruction >> 8) & 15) as usize;
            let rm = (instruction & 15) as usize;
            let signed = instruction & (1 << 22) != 0;
            let accumulate = instruction & (1 << 21) != 0;
            let multiplier = self.registers[rs];
            let mut result = if signed {
                (i64::from(self.registers[rm] as i32) * i64::from(multiplier as i32)) as u64
            } else {
                u64::from(self.registers[rm]) * u64::from(multiplier)
            };
            if accumulate {
                result = result.wrapping_add(
                    (u64::from(self.registers[hi]) << 32) | u64::from(self.registers[lo]),
                );
            }
            self.registers[lo] = result as u32;
            self.registers[hi] = (result >> 32) as u32;
            if instruction & (1 << 20) != 0 {
                self.cpsr = (self.cpsr & !(CPSR_N | CPSR_Z))
                    | if result >> 63 != 0 { CPSR_N } else { 0 }
                    | if result == 0 { CPSR_Z } else { 0 };
            }
            let iterations = [8_u32, 16, 24]
                .into_iter()
                .find(|bits| {
                    let upper = multiplier >> bits;
                    upper == 0 || (signed && (multiplier as i32) >> bits == -1)
                })
                .map_or(4, |bits| bits / 8);
            bus.idle(u64::from(iterations) + 1 + u64::from(accumulate));
            return true;
        }
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

    /// SWP snapshots the source before the read so overlapping operands retain
    /// the stored value. The word read rotates; the write aligns down.
    fn execute_swap<B: CpuBus>(
        &mut self,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0fb0_0ff0 != 0x0100_0090 {
            return Ok(false);
        }
        let target = self.registers[((instruction >> 16) & 15) as usize];
        let source = self.registers[(instruction & 15) as usize];
        let rd = ((instruction >> 12) & 15) as usize;
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        self.registers[rd] = if instruction & (1 << 22) != 0 {
            let value = bus.read8(target, access)?;
            bus.write8(target, source as u8, access)?;
            u32::from(value)
        } else {
            let value = bus
                .read32(target & !3, access)?
                .rotate_right((target & 3) * 8);
            bus.write32(store_address(target, 3), source, access)?;
            value
        };
        bus.idle(1);
        Ok(true)
    }

    /// Handles immediate and shifted-register byte/word transfers. Word loads
    /// rotate unaligned data; stores align down. Load/base overlap suppresses
    /// writeback, and a loaded PC retains state while refilling the pipeline.
    fn execute_word_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0C00_0000 != 0x0400_0000 {
            return Ok(false);
        }
        let byte = instruction & (1 << 22) != 0;
        let rn = ((instruction >> 16) & 15) as usize;
        let rd = ((instruction >> 12) & 15) as usize;
        let pre = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0 || !pre;
        let load = instruction & (1 << 20) != 0;
        if writeback && rn == 15 {
            return Ok(false);
        }
        let base = if rn == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[rn]
        };
        let offset = if instruction & (1 << 25) != 0 {
            if instruction & (1 << 4) != 0 {
                return Ok(false);
            }
            let rm = (instruction & 15) as usize;
            let value = if rm == 15 {
                address.wrapping_add(8)
            } else {
                self.registers[rm]
            };
            self.shift_immediate(value, (instruction >> 5) & 3, (instruction >> 7) & 31)
                .0
        } else {
            instruction & 0xfff
        };
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
            let value = if rd == 15 {
                address.wrapping_add(12)
            } else {
                self.registers[rd]
            };
            if byte {
                bus.write8(target, value as u8, access)?;
            } else {
                bus.write32(store_address(target, 3), value, access)?;
            }
        }
        if writeback && !(load && rn == rd) {
            self.registers[rn] = updated;
        }
        if load && rd == 15 {
            self.branch(bus, self.registers[15], false)?;
        }
        Ok(true)
    }

    /// User-bank transfers bypass only registers banked by the active mode.
    fn read_user_register(&self, register: usize) -> u32 {
        if (8..15).contains(&register)
            && (Self::status_bank(self.cpsr) == Some(0)
                || (register >= 13 && Self::status_bank(self.cpsr).is_some()))
        {
            self.user_bank[register - 8]
        } else {
            self.registers[register]
        }
    }

    /// Writes the user bank without changing CPSR or the currently visible bank.
    fn write_user_register(&mut self, register: usize, value: u32) {
        if (8..15).contains(&register)
            && (Self::status_bank(self.cpsr) == Some(0)
                || (register >= 13 && Self::status_bank(self.cpsr).is_some()))
        {
            self.user_bank[register - 8] = value;
        } else {
            self.registers[register] = value;
        }
    }

    /// Shares ARM/Thumb block-transfer semantics: ascending register order,
    /// aligned bus beats with unaligned base writeback, ARM7 empty-list behavior,
    /// and user-bank transfers or SPSR restoration when the S bit is present.
    fn execute_block_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E00_0000 != 0x0800_0000 {
            return Ok(false);
        }
        let rn = ((instruction >> 16) & 15) as usize;
        let raw_list = instruction & 0xffff;
        let list = if raw_list == 0 { 1 << 15 } else { raw_list };
        let user = instruction & (1 << 22) != 0;
        let load = instruction & (1 << 20) != 0;
        let writeback = instruction & (1 << 21) != 0;
        if rn == 15 {
            return Ok(false);
        }
        let bytes = if raw_list == 0 {
            64
        } else {
            list.count_ones() * 4
        };
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
                let value = bus.read32(target & !3, access)?;
                if user && list & (1 << 15) == 0 {
                    self.write_user_register(register, value);
                } else {
                    self.registers[register] = value;
                }
            } else {
                let value = if register == 15 {
                    address.wrapping_add(if self.cpsr & CPSR_T != 0 { 6 } else { 12 })
                } else if register == rn && writeback && list.trailing_zeros() as usize != rn {
                    updated
                } else if user {
                    self.read_user_register(register)
                } else {
                    self.registers[register]
                };
                bus.write32(store_address(target, 3), value, access)?;
            }
            target = target.wrapping_add(4);
            sequential = true;
        }
        if writeback && !(load && list & (1 << rn) != 0) {
            self.registers[rn] = updated;
        }
        if load {
            bus.idle(1);
            if list & (1 << 15) != 0 {
                if user && let Some(bank) = Self::status_bank(self.cpsr) {
                    self.set_status(self.saved_status[bank]);
                }
                self.branch(bus, self.registers[15], false)?;
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
        if instruction & 0x0e00_0090 != 0x0000_0090 || instruction & 0x60 == 0 {
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
            self.registers[source] =
                self.load_halfword_signed(bus, target, (instruction >> 5) & 3, access)?;
            bus.idle(1);
        } else {
            bus.write16(
                store_address(target, 1),
                self.registers[source] as u16,
                access,
            )?;
        }
        if (!pre || writeback) && !(instruction & (1 << 20) != 0 && base_register == source) {
            self.registers[base_register] = updated;
        }
        Ok(true)
    }

    /// ARM7 odd LDRH rotates the zero-extended halfword; odd LDRSH becomes
    /// LDRSB. Both instruction sets share these data-bus semantics.
    fn load_halfword_signed<B: CpuBus>(
        &self,
        bus: &mut B,
        target: u32,
        kind: u32,
        access: Access,
    ) -> Result<u32, CoreError> {
        match kind {
            1 => Ok(u32::from(bus.read16(target & !1, access)?).rotate_right((target & 1) * 8)),
            2 => Ok(bus.read8(target, access)? as i8 as i32 as u32),
            3 if target & 1 != 0 => Ok(bus.read8(target, access)? as i8 as i32 as u32),
            3 => Ok(bus.read16(target, access)? as i16 as i32 as u32),
            _ => Err(CoreError::InvalidTransferKind { kind }),
        }
    }
}

struct StepOutcome {
    address: u32,
}

/// Cartridge bus ownership and the eight-halfword opcode FIFO. Addresses stay
/// in their waitstate window; changing windows cannot reuse another bank's fill.
#[derive(Default)]
struct GamePak {
    /// Contiguous address eligible for the next sequential cartridge access.
    next_access: Option<u32>,
    /// Distinguishes opcode fetch continuity from a cartridge data transfer.
    fetch_stream: bool,
    /// Address of the next halfword the CPU can consume from the opcode FIFO.
    head: Option<u32>,
    /// Completed halfwords available to the CPU, bounded by the eight-entry FIFO.
    buffered: u32,
    /// Cycles already spent fetching the next unbuffered halfword.
    progress: u64,
}

impl GamePak {
    /// WAITCNT fields contain wait cycles; each beat also needs a transfer cycle.
    fn beat_cycles(waitcnt: u16, address: u32, sequential: bool) -> u64 {
        let bank = ((address >> 25) - 4) as usize;
        let first_shift = [2, 5, 8][bank];
        let second_shift = [4, 7, 10][bank];
        if sequential && address & 0x1ffff != 0 {
            if waitcnt & (1 << second_shift) != 0 {
                2
            } else {
                [3, 5, 9][bank]
            }
        } else {
            [5, 4, 3, 9][((waitcnt >> first_shift) & 3) as usize]
        }
    }

    /// Fills only while the cartridge bus is free, retaining an unfinished beat.
    fn fill(&mut self, waitcnt: u16, cycles: u64) {
        if waitcnt & 0x4000 == 0 {
            return;
        }
        let Some(head) = self.head else {
            return;
        };
        self.progress += cycles;
        while self.buffered < 8 {
            let address = head + self.buffered * 2;
            if !(0x08000000..0x0e000000).contains(&address) {
                self.progress = 0;
                break;
            }
            let cost = Self::beat_cycles(waitcnt, address, true);
            if self.progress < cost {
                break;
            }
            self.progress -= cost;
            self.buffered += 1;
        }
        if self.buffered == 8 {
            self.progress = 0;
        }
    }

    /// Consumes instruction beats without letting data reads enter the FIFO.
    /// A matching FIFO address survives CPU accesses to internal RAM; cartridge
    /// data, redirects, and WAITCNT writes discard the old stream instead.
    fn access(&mut self, waitcnt: u16, address: u32, width: usize, access: Access) -> u64 {
        let fetch = matches!(access.kind, AccessKind::Fetch);
        let enabled = waitcnt & 0x4000 != 0;
        let hit = fetch && enabled && self.head == Some(address);
        let sequential =
            access.sequential && self.next_access == Some(address) && self.fetch_stream == fetch;
        if !hit {
            self.head = None;
            self.buffered = 0;
            self.progress = 0;
        }
        let mut cycles = 0;
        for beat in 0..(width / 2).max(1) as u32 {
            let current = address + beat * 2;
            if hit && self.buffered != 0 {
                self.buffered -= 1;
            } else {
                let cost = Self::beat_cycles(waitcnt, current, beat != 0 || sequential || hit);
                cycles += cost.saturating_sub(self.progress);
                self.progress = 0;
            }
        }
        let next = address + (width as u32).max(2);
        self.next_access = Some(next);
        self.fetch_stream = fetch;
        self.head = (fetch && enabled).then_some(next);
        if cycles == 0 {
            // A FIFO hit occupies one CPU cycle while the cartridge keeps filling.
            self.fill(waitcnt, 1);
            1
        } else {
            cycles
        }
    }
}

/// A latched DMA descriptor. Programmed registers remain distinct from these
/// internal pointers so repeat transfers can reload count/destination without
/// rewinding the progressing source.
#[derive(Default)]
struct DmaTransfer {
    source: u32,
    destination: u32,
    initial_destination: u32,
    count: u32,
    remaining: u32,
    control: u16,
    active: bool,
    sequential: bool,
}

impl DmaTransfer {
    #[inline]
    fn enabled_for(&self, timing: u16) -> bool {
        self.control & 0x8000 != 0 && (self.control >> 12) & 3 == timing
    }
}

struct System {
    dma: [DmaTransfer; 4],
    audio: audio::Audio,
    /// HALT stops instruction retirement while hardware time continues.
    halted: bool,
    buttons: ButtonState,
    inputs: VecDeque<InputEvent>,
    rom: Vec<u8>,
    /// Cartridge-owned SRAM persists through CPU reset; host storage is separate.
    sram: Option<crate::sram::Sram>,
    /// Flash commands and bytes share the cartridge lifecycle, not host storage.
    flash: Option<crate::flash::Flash>,
    /// Serial cartridge backup occupies the EEPROM window in ROM waitstate two.
    eeprom: Option<eeprom::Eeprom>,
    /// Explicitly mapped original test firmware; absent for normal ROM loads.
    test_firmware: bool,
    /// User-supplied firmware is owned separately from controlled diagnostics.
    bios: Option<Box<[u8; BIOS_SIZE]>>,
    /// Last successful opcode bus fetch in BIOS, including pipeline lookahead.
    bios_latch: u32,
    bios_enabled: bool,
    execution_address: u32,
    ewram: Vec<u8>,
    iwram: Vec<u8>,
    io: Vec<u8>,
    /// Shared BG/OBJ color RAM; mode 4 indexes the first 256 entries.
    palette: Vec<u8>,
    vram: Vec<u8>,
    /// 128 eight-byte object entries; the fourth halfword also stores affine data.
    oam: Vec<u8>,
    display: Display,
    cycles: u64,
    gamepak: GamePak,
    /// Last driven word supplies otherwise unmapped data reads.
    open_bus: u32,
    /// Instruction prefetch remains independent of the DMA/data bus latch.
    cpu_open_bus: u32,
}

impl System {
    fn new() -> Self {
        let mut system = Self {
            dma: std::array::from_fn(|_| DmaTransfer::default()),
            audio: audio::Audio::new(),
            halted: false,
            buttons: ButtonState::default(),
            inputs: VecDeque::new(),
            rom: Vec::new(),
            sram: None,
            flash: None,
            eeprom: None,
            test_firmware: false,
            bios: None,
            bios_latch: 0,
            bios_enabled: false,
            execution_address: ROM_START,
            ewram: vec![0; 256 * 1024],
            iwram: vec![0; 32 * 1024],
            io: vec![0; IO_BYTES],
            palette: vec![0; PALETTE_BYTES],
            vram: vec![0; VRAM_BYTES],
            oam: vec![0; OAM_BYTES],
            display: Display::new(),
            cycles: 0,
            gamepak: GamePak::default(),
            open_bus: 0,
            cpu_open_bus: 0,
        };
        // Controlled cartridge startup supplies the BIOS-style identity transform.
        // Supplied-BIOS startup clears these values so firmware owns initialization.
        for offset in [0x20, 0x26, 0x30, 0x36] {
            system.io[offset..offset + 2].copy_from_slice(&256i16.to_le_bytes());
        }
        system.refresh_status();
        system
    }

    /// HALT observes enabled pending sources independently of IME and CPSR.I.
    fn pending_interrupts(&self) -> u16 {
        u16::from_le_bytes([self.io[0x200], self.io[0x201]])
            & u16::from_le_bytes([self.io[0x202], self.io[0x203]])
    }

    /// KEYCNT evaluates active-low KEYINPUT as pressed selection bits. Requests
    /// latch in IF independently of IE/IME; the shared IRQ path decides delivery.
    fn evaluate_keypad(&mut self) {
        let control = u16::from_le_bytes([self.io[0x132], self.io[0x133]]);
        let selected = control & 0x03ff;
        let pressed = self.buttons.0 & selected;
        let matched = if control & 0x8000 != 0 {
            selected != 0 && pressed == selected
        } else {
            pressed != 0
        };
        if control & 0x4000 != 0 && matched {
            self.io[0x203] |= 0x10;
        }
    }

    /// HALT must stop at input, audio, and display-DMA request boundaries rather
    /// than advancing past a timed transfer that affects the next visible line.
    fn next_wake_event(&self) -> u64 {
        let mut display = self.next_vblank();

        if self.dma.iter().any(|dma| dma.enabled_for(2)) {
            display = display.min(self.next_visible_hblank());
        }

        if self.io[4] & 0x20 != 0
            && let Some(vcount) = self.next_vcount_match()
        {
            display = display.min(vcount);
        }

        let display = display.min(self.audio.next_event(self.cycles));

        self.inputs
            .front()
            .map_or(display, |event| display.min(event.cycle.0))
    }

    /// HBlank DMA starts after the visible pixel interval on visible lines.
    fn next_hblank(&self) -> u64 {
        let edge = self.cycles / CYCLES_PER_SCANLINE * CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE;
        if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_SCANLINE
        }
    }

    /// Returns the next visible-line HBlank that may issue an ordinary timed
    /// DMA request. HBlank continues during VBlank, but those blank lines must
    /// not consume entries from a repeated raster table.
    fn next_visible_hblank(&self) -> u64 {
        let edge = self.next_hblank();
        let line = edge / CYCLES_PER_SCANLINE % SCANLINES_PER_FRAME;

        if line < SCREEN_HEIGHT as u64 {
            edge
        } else {
            (edge / CYCLES_PER_FRAME + 1) * CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE
        }
    }

    /// Returns the next scanline boundary matching DISPSTAT's VCOUNT compare.
    ///
    /// VCOUNT only ranges from 0 through 227. Compare values outside that range
    /// cannot match. An IRQ is generated on the transition into the matching
    /// scanline, not repeatedly while execution remains within that scanline.
    fn next_vcount_match(&self) -> Option<u64> {
        let compare = u64::from(self.io[5]);

        if compare >= SCANLINES_PER_FRAME {
            return None;
        }

        let frame_start = self.cycles / CYCLES_PER_FRAME * CYCLES_PER_FRAME;
        let edge = frame_start + compare * CYCLES_PER_SCANLINE;

        Some(if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_FRAME
        })
    }

    /// A low-to-high enable transition latches the descriptor. DMA0-2 use
    /// 14-bit counts and DMA3 uses 16 bits. DMA0 source and DMA0-2 destination
    /// addresses use 27 bits; the other source/destination ranges use 28 bits.
    fn configure_dma(&mut self, channel: usize, value: u16) {
        let base = 0xb0 + channel * 12;
        let sound = matches!(channel, 1 | 2) && value & 0x3000 == 0x3000;
        let old = self.dma[channel].control;
        self.dma[channel].control = value;
        if value & 0x8000 == 0 {
            self.dma[channel].active = false;
            return;
        }

        if old & 0x8000 != 0 {
            return;
        }

        let width = if sound || value & 0x0400 != 0 { 4 } else { 2 };
        let source_mask = if channel == 0 {
            0x07ff_ffff
        } else {
            0x0fff_ffff
        };
        let destination_mask = if channel == 3 {
            0x0fff_ffff
        } else {
            0x07ff_ffff
        };

        let source = u32::from_le_bytes([
            self.io[base],
            self.io[base + 1],
            self.io[base + 2],
            self.io[base + 3],
        ]);

        let destination = u32::from_le_bytes([
            self.io[base + 4],
            self.io[base + 5],
            self.io[base + 6],
            self.io[base + 7],
        ]);

        self.dma[channel].source = source & source_mask & !(width - 1);

        self.dma[channel].destination = destination & destination_mask & !(width - 1);
        self.dma[channel].initial_destination = self.dma[channel].destination;

        let programmed_count = u16::from_le_bytes([self.io[base + 8], self.io[base + 9]]);
        let latched_count = if channel == 3 {
            u32::from(programmed_count)
        } else {
            u32::from(programmed_count & 0x3fff)
        };

        self.dma[channel].count = if sound {
            // FIFO DMA always transfers four 32-bit units per refill request.
            4
        } else if latched_count == 0 {
            if channel == 3 { 0x1_0000 } else { 0x4000 }
        } else {
            latched_count
        };
        self.dma[channel].remaining = self.dma[channel].count;

        // Immediate transfers own the bus now; timed transfers await a request.
        self.dma[channel].active = value & 0x3000 == 0;
        self.dma[channel].sequential = false;
    }

    /// Services one read/write beat, preserving state between host deadlines.
    /// CPU execution and IRQ entry wait for DMA to release bus ownership. Every
    /// memory access advances scanout and requests through the ordinary bus path.
    fn dma_beat(&mut self, channel: usize) -> Result<(), CoreError> {
        let control = self.dma[channel].control;
        let sound = matches!(channel, 1 | 2) && control & 0x3000 == 0x3000;
        let width = if sound || control & 0x400 != 0 { 4 } else { 2 };
        if !self.dma[channel].sequential {
            self.gamepak = GamePak::default();
            self.advance_time(2);
        }
        if channel == 3
            && width == 2
            && !self.dma[channel].sequential
            && self.eeprom_address(self.dma[channel].destination)
        {
            let count = self.dma[channel].count;

            if let Some(eeprom) = self.eeprom.as_mut() {
                eeprom.begin_dma(count);
            }
        }
        let access = Access {
            kind: AccessKind::Data,
            sequential: self.dma[channel].sequential,
        };
        if width == 4 {
            let value = self.read32_impl(self.dma[channel].source, access)?;
            self.write32_impl(self.dma[channel].destination, value, access)?;
        } else {
            let value = self.read16_impl(self.dma[channel].source, access)?;
            self.write16_impl(self.dma[channel].destination, value, access)?;
        }
        let advance = |address: u32, mode: u16| match mode {
            1 => address.wrapping_sub(width),
            2 => address,
            _ => address.wrapping_add(width),
        };
        self.dma[channel].source = advance(self.dma[channel].source, (control >> 7) & 3);
        if !sound {
            self.dma[channel].destination =
                advance(self.dma[channel].destination, (control >> 5) & 3);
        }
        self.dma[channel].sequential = true;
        self.dma[channel].remaining -= 1;
        if self.dma[channel].remaining == 0 {
            self.dma[channel].active = false;
            self.gamepak = GamePak::default();
            if control & 0x4000 != 0 {
                self.io[0x203] |= 1 << channel;
            }
            if control & 0x200 != 0 && control & 0x3000 != 0 {
                self.dma[channel].remaining = self.dma[channel].count;
                if control & 0x60 == 0x60 {
                    self.dma[channel].destination = self.dma[channel].initial_destination;
                }
                self.dma[channel].sequential = false;
            } else {
                self.dma[channel].control &= !0x8000;
                self.io[0xb0 + channel * 12 + 11] &= !0x80;
            }
        }
        Ok(())
    }

    /// Absolute next VBlank edge; sleeping callers clip it to their own deadline.
    fn next_vblank(&self) -> u64 {
        let edge = self.cycles / CYCLES_PER_FRAME * CYCLES_PER_FRAME
            + SCREEN_HEIGHT as u64 * CYCLES_PER_SCANLINE;
        if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_FRAME
        }
    }

    /// Refreshes read-only register bits from hardware time, even during blank lines.
    /// Interrupt requests are latched at event crossings, not status reads.
    fn refresh_status(&mut self) {
        let line = (self.cycles / CYCLES_PER_SCANLINE % SCANLINES_PER_FRAME) as u16;
        let control = u16::from_le_bytes([self.io[4], self.io[5]]) & 0xff38;
        let flags = u16::from((160..227).contains(&line))
            | (u16::from(self.cycles % CYCLES_PER_SCANLINE >= HBLANK_FLAG_CYCLE) << 1)
            | (u16::from(line == control >> 8) << 2);
        self.io[4..6].copy_from_slice(&(control | flags).to_le_bytes());
        self.io[6..8].copy_from_slice(&line.to_le_bytes());
        self.audio.refresh_timers(&mut self.io);
        self.audio.refresh_controls(&mut self.io);
        self.io[0x130..0x132].copy_from_slice(&(!self.buttons.0 & 0x03ff).to_le_bytes());
    }

    fn read16_impl(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }
        self.charge(address, 2, access);
        if self.eeprom_address(address)
            && matches!(access.kind, AccessKind::Data)
            && let Some(eeprom) = self.eeprom.as_mut()
        {
            let value = eeprom.read();
            self.open_bus = u32::from(value) * 0x0001_0001;
            return Ok(value);
        }
        if let Some(value) = self.backup_read(address) {
            let value = u16::from(value) * 0x0101;
            self.open_bus = u32::from(value) * 0x0001_0001;
            return Ok(value);
        }
        if let Some(word) = self.bios_read(address, access) {
            return Ok((word >> ((address & 2) * 8)) as u16);
        }
        let value = match self.read_bytes(address, 2) {
            Ok(bytes) => u16::from_le_bytes([bytes[0], bytes[1]]),
            Err(_) if matches!(access.kind, AccessKind::Data) => {
                (self.unmapped_read() >> ((address & 2) * 8)) as u16
            }
            Err(error) => return Err(error),
        };
        self.open_bus = u32::from(value) * 0x0001_0001;
        Ok(value)
    }

    fn read32_impl(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);
        if let Some(value) = self.backup_read(address) {
            self.open_bus = u32::from(value) * 0x0101_0101;
            return Ok(self.open_bus);
        }
        if let Some(word) = self.bios_read(address, access) {
            return Ok(word);
        }
        let value = match self.read_bytes(address, 4) {
            Ok(bytes) => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Err(_) if matches!(access.kind, AccessKind::Data) => self.unmapped_read(),
            Err(error) => return Err(error),
        };
        self.open_bus = value;
        Ok(value)
    }

    fn write16_impl(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError> {
        // Backup memory is an eight-bit bus: retain the original address to
        // select the corresponding source byte even for an unaligned store.
        if (0x0e000000..0x10000000).contains(&address)
            && (self.sram.is_some() || self.flash.is_some())
        {
            self.charge(address, 2, access);
            self.backup_write(address, value.rotate_right((address & 1) * 8) as u8);
            return Ok(());
        }
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }

        self.charge(address, 2, access);

        self.write_halfword(address, value)
    }

    /// Applies a bus beat after its access time has synchronized scanout. Word
    /// transfers reuse this path without charging the hardware clock twice.
    fn write_halfword(&mut self, address: u32, value: u16) -> Result<(), CoreError> {
        if self.bios_enabled && address < BIOS_SIZE as u32 {
            return Ok(());
        }

        if self.eeprom_address(address) {
            if let Some(eeprom) = self.eeprom.as_mut() {
                eeprom.write(value);
            }

            return Ok(());
        }

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
            if matches!(offset, 0x50 | 0x52 | 0x54) {
                let stored = if offset == 0x50 {
                    // BLDCNT: bits 14..15 are unused.
                    value & 0x3fff
                } else if offset == 0x52 {
                    // BLDALPHA: EVA and EVB are five-bit register fields.
                    value & 0x1f1f
                } else {
                    // BLDY: effective EVY saturates at 16.
                    (value & 0x001f).min(16)
                };

                self.io[range].copy_from_slice(&stored.to_le_bytes());
                return Ok(());
            }

            if matches!(offset, 0xba | 0xc6 | 0xd2 | 0xde) {
                let channel = (offset - 0xba) / 12;
                // Bits 0..4 are unused on every channel; Game Pak DRQ is DMA3-only.
                let control = value & if channel < 3 { 0xf7e0 } else { 0xffe0 };
                self.io[range].copy_from_slice(&control.to_le_bytes());
                self.configure_dma(channel, control);
                return Ok(());
            }
            if (0x100..0x110).contains(&offset) {
                self.audio.write_timer(offset, value);
                self.audio.refresh_timers(&mut self.io);
                return Ok(());
            }
            if matches!(
                offset,
                0x60 | 0x62 | 0x64 | 0x68 | 0x6c | 0x70 | 0x72 | 0x74 | 0x78 | 0x7c | 0x90
                    ..=0x9e | 0x80 | 0x82 | 0x84 | 0x88
            ) {
                self.audio.write_control(offset, value);
                if offset == 0x84 && value & 0x80 == 0 {
                    self.io[0x60..0x82].fill(0);
                }
                self.audio.refresh_controls(&mut self.io);
                return Ok(());
            }
            if matches!(offset, 0xa0 | 0xa2 | 0xa4 | 0xa6) {
                self.audio.push_channel(usize::from(offset >= 0xa4), &bytes);
                return Ok(());
            }
            if matches!(offset, 6 | 0x86 | 0x8a | 0x130) {
                return Ok(());
            }
            if offset == 0x132 {
                self.io[range].copy_from_slice(&(value & 0xc3ff).to_le_bytes());
                self.evaluate_keypad();
                return Ok(());
            }
            if matches!(offset, 0x200 | 0x202 | 0x208) {
                let old = u16::from_le_bytes([self.io[offset], self.io[offset + 1]]);
                let value = match offset {
                    0x202 => old & !(value & 0x3fff),
                    0x208 => value & 1,
                    _ => value & 0x3fff,
                };
                self.io[range].copy_from_slice(&value.to_le_bytes());
                return Ok(());
            }
            if offset == 0x300 {
                self.io[0x300] = value as u8;
                self.halted = value & 0x8000 == 0;
                return Ok(());
            }
            if offset == 0x204 {
                self.gamepak.head = None;
                self.gamepak.buffered = 0;
                self.gamepak.progress = 0;
                self.io[range].copy_from_slice(&(value & 0x5fff).to_le_bytes());
                return Ok(());
            }
            self.io[range].copy_from_slice(&bytes);
            self.display.write_reference(offset, &self.io);
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

        if let Some(range) = vram_range(address, 2) {
            self.vram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = ram_range(address, OAM_START, OAM_BYTES, 2) {
            self.oam[range].copy_from_slice(&bytes);
            return Ok(());
        }

        // Ordinary writes to cartridge ROM space do not trap on the GBA.
        // Cartridge peripherals such as EEPROM must intercept their addresses
        // before reaching this fallback. Future GPIO/RTC handling must do the same.
        if (0x0800_0000..0x0e00_0000).contains(&address) {
            return Ok(());
        }

        Err(CoreError::UnmappedAddress { address, width: 2 })
    }

    fn write32_impl(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError> {
        // Backup memory is an eight-bit bus: retain the original address to
        // select the corresponding source byte even for an unaligned store.
        if (0x0e000000..0x10000000).contains(&address)
            && (self.sram.is_some() || self.flash.is_some())
        {
            self.charge(address, 4, access);
            self.backup_write(address, value.rotate_right((address & 3) * 8) as u8);
            return Ok(());
        }
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);

        self.write_halfword(address, value as u16)?;
        self.write_halfword(address.wrapping_add(2), (value >> 16) as u16)
    }

    /// EEPROM uses all of region 0D for ROMs up to 16 MiB, and only the
    /// final 256 bytes for larger cartridges. Other ROM mirrors remain intact.
    fn eeprom_address(&self, address: u32) -> bool {
        self.eeprom.is_some()
            && (0x0d000000..0x0e000000).contains(&address)
            && (self.rom.len() <= 0x01000000 || address >= 0x0dffff00)
    }

    /// The eight-bit save bus delegates mirroring and commands to the selected chip.
    fn backup_read(&mut self, address: u32) -> Option<u8> {
        if !(0x0e000000..0x10000000).contains(&address) {
            return None;
        }
        self.sram
            .as_ref()
            .map(|sram| sram.read(address))
            .or_else(|| self.flash.as_mut().map(|flash| flash.read(address)))
    }

    fn backup_write(&mut self, address: u32, value: u8) -> bool {
        if !(0x0e000000..0x10000000).contains(&address) {
            return false;
        }
        if let Some(sram) = &mut self.sram {
            sram.write(address, value);
            true
        } else if let Some(flash) = &mut self.flash {
            flash.write(address, value);
            true
        } else {
            false
        }
    }

    /// DMA observes the data bus; CPU unmapped reads observe instruction prefetch.
    fn unmapped_read(&self) -> u32 {
        if self.dma.iter().any(|dma| dma.active) {
            self.open_bus
        } else {
            self.cpu_open_bus
        }
    }

    /// BIOS protection depends on CPU execution context, including DMA reads.
    /// Width-specific callers select the corresponding byte lanes from this word.
    fn bios_read(&mut self, address: u32, access: Access) -> Option<u32> {
        if !self.bios_enabled {
            return None;
        }
        let bios = self.bios.as_ref()?;
        if address >= BIOS_SIZE as u32 {
            return None;
        }
        if self.execution_address < BIOS_SIZE as u32 {
            let offset = address as usize & !3;
            let word = u32::from_le_bytes([
                bios[offset],
                bios[offset + 1],
                bios[offset + 2],
                bios[offset + 3],
            ]);
            if matches!(access.kind, AccessKind::Fetch) {
                self.bios_latch = word;
            }
            Some(word)
        } else {
            Some(self.bios_latch)
        }
    }

    fn read_bytes(&self, address: u32, width: usize) -> Result<&[u8], CoreError> {
        if self.test_firmware
            && let Some(range) = range_for(address, 0, TEST_FIRMWARE.len(), width)
        {
            return Ok(&TEST_FIRMWARE[range]);
        }
        if (0x08000000..0x0e000000).contains(&address)
            && let Some(range) = range_for(address & 0x01ffffff, 0, self.rom.len(), width)
        {
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
        if let Some(range) = vram_range(address, width) {
            return Ok(&self.vram[range]);
        }

        if let Some(range) = ram_range(address, OAM_START, OAM_BYTES, width) {
            return Ok(&self.oam[range]);
        }
        Err(CoreError::UnmappedAddress { address, width })
    }

    /// Charges timing at the bus boundary, once per CPU access. Internal memory
    /// leaves the cartridge free to prefetch; SRAM and ROM accesses own that bus.
    fn charge(&mut self, address: u32, width: usize, access: Access) {
        let waitcnt = u16::from_le_bytes([self.io[0x204], self.io[0x205]]);
        let beats = (width / 2).max(1) as u64;
        let cartridge = (0x08000000..0x10000000).contains(&address);
        let cycles = if (0x08000000..0x0e000000).contains(&address) {
            self.gamepak.access(waitcnt, address, width, access)
        } else if (0x0e000000..0x10000000).contains(&address) {
            self.gamepak = GamePak::default();
            [5, 4, 3, 9][(waitcnt & 3) as usize]
        } else if ram_range(address, EWRAM_START, 256 * 1024, 1).is_some() {
            3 * beats
        } else if ram_range(address, IWRAM_START, 32 * 1024, 1).is_some()
            || range_for(address, IO_START, IO_BYTES, 1).is_some()
            || ram_range(address, OAM_START, OAM_BYTES, 1).is_some()
        {
            1
        } else {
            beats
        };
        if !cartridge {
            self.gamepak.fill(waitcnt, cycles);
        }
        self.advance_time(cycles);
    }

    /// Advances display events before any access changes the state observed by scanout.
    fn advance_time(&mut self, cycles: u64) {
        let deadline = self.cycles.saturating_add(cycles);

        loop {
            let mut target = deadline.min(self.audio.next_event(self.cycles));

            if self.io[4] & 0x20 != 0
                && let Some(vcount) = self.next_vcount_match()
            {
                target = target.min(vcount);
            }

            self.advance_devices(target);

            if self.cycles >= deadline {
                break;
            }
        }
    }

    fn advance_devices(&mut self, target: u64) {
        // Raise timed requests before CPU execution can continue. Each channel
        // still transfers one bus beat at a time through Machine::step(), where
        // the existing channel order provides hardware priority.
        let vblank = self.next_vblank();
        let hblank = self.next_visible_hblank();
        let vcount = self.next_vcount_match();

        let vblank_due = vblank <= target;
        let hblank_due = hblank <= target;
        let vcount_due = vcount.is_some_and(|edge| edge <= target);

        for dma in &mut self.dma {
            if dma.active {
                continue;
            }

            if (vblank_due && dma.enabled_for(1)) || (hblank_due && dma.enabled_for(2)) {
                dma.active = true;
            }
        }

        // Repeated reads in VBlank must not regenerate an acknowledged request.
        if vblank_due && self.io[4] & 8 != 0 {
            self.io[0x202] |= 1;
        }

        // VCOUNT requests IRQ exactly when VCOUNT enters the programmed comparison
        // scanline. IF retains the request until guest software acknowledges bit 2.
        if vcount_due && self.io[4] & 0x20 != 0 {
            self.io[0x202] |= 1 << 2;
        }
        while self
            .inputs
            .front()
            .is_some_and(|event| event.cycle.0 <= target)
        {
            let Some(event) = self.inputs.pop_front() else {
                break;
            };

            if self.buttons.set(event.button, event.pressed) {
                self.evaluate_keypad();
            }
        }
        self.display
            .synchronize_to(target, &self.vram, &self.palette, &self.io, &self.oam);
        let (irq, refill) = self.audio.advance(target - self.cycles, target);
        let flags = u16::from_le_bytes([self.io[0x202], self.io[0x203]]) | irq;
        self.io[0x202..0x204].copy_from_slice(&flags.to_le_bytes());
        // Both sound DMA channels independently follow their latched FIFO
        // destination. Arbitration retains the ordinary DMA1-before-DMA2 order.
        for dma in &mut self.dma[1..=2] {
            if refill[usize::from(dma.destination == IO_START + 0xa4)]
                && dma.control & 0xb000 == 0xb000
                && matches!(dma.destination, 0x040000a0 | 0x040000a4)
                && !dma.active
            {
                dma.active = true;
            }
        }
        self.cycles = target;
        self.refresh_status();
    }
}

impl CpuBus for System {
    fn set_cpu_open_bus(&mut self, pipeline: [u32; 2], address: u32, thumb: bool) {
        self.cpu_open_bus = if !thumb {
            pipeline[1]
        } else {
            let fetch_address = address.wrapping_add(4);
            match fetch_address >> 24 {
                // These regions have a 32-bit instruction bus. BIOS fetches
                // already captured the complete aligned word in the protected latch.
                0 if self.bios_enabled => self.bios_latch,
                7 => self
                    .read_bytes(fetch_address & !3, 4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .unwrap_or(pipeline[1] * 0x0001_0001),
                3 if fetch_address & 2 != 0 => (pipeline[1] << 16) | pipeline[0],
                3 => pipeline[1] | (pipeline[0] << 16),
                _ => pipeline[1] * 0x00010001,
            }
        };
    }

    fn set_execution_address(&mut self, address: u32) {
        self.execution_address = address;
    }
    fn restart_fetch(&mut self) {
        self.gamepak = GamePak::default();
    }

    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError> {
        self.charge(address, 1, access);
        if let Some(value) = self.backup_read(address) {
            return Ok(value);
        }
        if let Some(word) = self.bios_read(address, access) {
            return Ok((word >> ((address & 3) * 8)) as u8);
        }
        match self.read_bytes(address, 1) {
            Ok(bytes) => Ok(bytes[0]),
            Err(_) if matches!(access.kind, AccessKind::Data) => {
                Ok((self.unmapped_read() >> ((address & 3) * 8)) as u8)
            }
            Err(error) => Err(error),
        }
    }

    /// Video memory has a 16-bit write bus: palette and BG bytes are duplicated,
    /// while OBJ VRAM and OAM ignore byte stores. RAM bytes retain ordinary semantics.
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError> {
        self.charge(address, 1, access);
        if self.backup_write(address, value) {
            return Ok(());
        }

        if self.bios_enabled && address < BIOS_SIZE as u32 {
            // The BIOS is ROM. Stores occupy the bus but do not modify it and do
            // not raise a CPU-visible memory fault.
            return Ok(());
        }

        // RegisterRamReset writes a byte to this undocumented write-only
        // location. Accept the timed access without backing unused MMIO space;write8(
        // neighboring addresses retain their existing unmapped behavior.
        if address == BIOS_UNDOCUMENTED_IO_410 {
            return Ok(());
        }
        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), 1) {
            self.ewram[range.start] = value;
        } else if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), 1) {
            self.iwram[range.start] = value;
        } else if let Some(range) = range_for(address, IO_START, self.io.len(), 1) {
            let offset = range.start;
            if (0xa0..0xa8).contains(&offset) {
                self.audio
                    .push_channel(usize::from(offset >= 0xa4), &[value]);
            } else if offset == 0x301 {
                // STOP remains outside this fixture; HALT uses bit 7 clear.
                self.halted = value & 0x80 == 0;
            } else if offset == 0x300 {
                self.io[offset] = value;
            } else {
                let aligned = offset & !1;
                let shift = (offset & 1) * 8;
                let old = self.audio.pulse_latch(aligned).unwrap_or_else(|| {
                    u16::from_le_bytes([self.io[aligned], self.io[aligned + 1]])
                });
                let merged = if aligned == 0x202 {
                    u16::from(value) << shift
                } else {
                    (old & !(0xff << shift)) | (u16::from(value) << shift)
                };
                self.write_halfword(address & !1, merged)?;
            }
        } else if palette_range(address, 1).is_some() {
            self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
        } else if let Some(range) = vram_range(address, 1) {
            let object_start = if self.display.control & 7 >= 3 {
                0x14000
            } else {
                0x10000
            };
            if range.start < object_start {
                self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
            }
        } else if (0x0800_0000..0x0e00_0000).contains(&address) {
            // ROM bus store with no selected cartridge peripheral.
            // The access consumes timing but has no storage effect.
            return Ok(());
        } else if ram_range(address, OAM_START, OAM_BYTES, 1).is_none() {
            return Err(CoreError::UnmappedAddress { address, width: 1 });
        }
        // OAM has a 32-bit bus but no byte-write enables: STRB is ignored,
        // including mirrors. Halfword and word accesses retain their values.
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
        let waitcnt = u16::from_le_bytes([self.io[0x204], self.io[0x205]]);
        self.gamepak.fill(waitcnt, cycles);
        self.advance_time(cycles);
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

/// Register snapshot for one text-background scanline. This is rebuilt at each
/// drawing boundary so guest writes need no tile or register cache invalidation.
struct TextBackground {
    control: u16,
    scroll_x: usize,
    scroll_y: usize,
    width: usize,
    height: usize,
}

impl TextBackground {
    fn from_registers(io: &[u8], background: usize) -> Self {
        let halfword = |offset| u16::from_le_bytes([io[offset], io[offset + 1]]);
        let control = halfword(8 + background * 2);
        Self {
            control,
            scroll_x: usize::from(halfword(0x10 + background * 4) & 0x1ff),
            scroll_y: usize::from(halfword(0x12 + background * 4) & 0x1ff),
            width: if control & (1 << 14) != 0 { 512 } else { 256 },
            height: if control & (1 << 15) != 0 { 512 } else { 256 },
        }
    }

    /// Resolves a texel through screen blocks, tile flips, and its palette.
    /// Color index zero is transparent regardless of the selected palette bank.
    fn pixel(&self, x: usize, y: usize, vram: &[u8], palette: &[u8]) -> Option<u16> {
        let x = (x + self.scroll_x) % self.width;
        let y = (y + self.scroll_y) % self.height;
        let block = x / 256 + (y / 256) * (self.width / 256);
        let map_base = usize::from((self.control >> 8) & 31) * 0x800;
        let map_offset = (map_base + block * 0x800 + ((y / 8 % 32) * 32 + x / 8 % 32) * 2) & 0xffff;
        let entry = u16::from_le_bytes([vram[map_offset], vram[map_offset + 1]]);
        let tile_x = if entry & (1 << 10) != 0 {
            7 - x % 8
        } else {
            x % 8
        };
        let tile_y = if entry & (1 << 11) != 0 {
            7 - y % 8
        } else {
            y % 8
        };
        let character_base = usize::from((self.control >> 2) & 3) * 0x4000;
        let tile = usize::from(entry & 0x3ff);
        let (color, bank) = if self.control & (1 << 7) != 0 {
            let offset = character_base + tile * 64 + tile_y * 8 + tile_x;
            // Text tiles cannot source the OBJ character region above 64 KiB.
            let color = *vram.get(..0x10000)?.get(offset)?;
            (usize::from(color), 0)
        } else {
            let offset = character_base + tile * 32 + tile_y * 4 + tile_x / 2;
            let packed = *vram.get(..0x10000)?.get(offset)?;
            let color = (packed >> ((tile_x % 2) * 4)) & 15;
            (usize::from(color), usize::from(entry >> 12) * 16)
        };
        if color == 0 {
            return None;
        }
        let offset = (bank + color) * 2;
        Some(u16::from_le_bytes([palette[offset], palette[offset + 1]]))
    }
}

/// Snapshot of an object and its optional signed 8.8 OAM transformation.
/// Source dimensions determine tile addressing; display bounds determine clipping.
struct Object {
    attr0: u16,
    attr1: u16,
    attr2: u16,
    width: usize,
    height: usize,
    matrix: Option<[i32; 4]>,
}

impl Object {
    fn from_oam(entry: &[u8], oam: &[u8]) -> Option<Self> {
        let attr0 = u16::from_le_bytes([entry[0], entry[1]]);
        let attr1 = u16::from_le_bytes([entry[2], entry[3]]);
        let attr2 = u16::from_le_bytes([entry[4], entry[5]]);
        // Bit 9 disables regular objects but doubles affine display bounds.
        // Object-window texels select masks; semitransparent effects arrive separately.
        let affine = attr0 & 0x100 != 0;
        if attr0 & 0x0c00 == 0x0c00 || (!affine && attr0 & 0x200 != 0) {
            return None;
        }
        let dimensions = match attr0 >> 14 {
            0 => [(8, 8), (16, 16), (32, 32), (64, 64)],
            1 => [(16, 8), (32, 8), (32, 16), (64, 32)],
            2 => [(8, 16), (8, 32), (16, 32), (32, 64)],
            _ => return None,
        };
        let (width, height) = dimensions[usize::from(attr1 >> 14)];
        Some(Self {
            attr0,
            attr1,
            attr2,
            width,
            height,
            matrix: affine.then(|| {
                let base = usize::from((attr1 >> 9) & 31) * 32;
                [6, 14, 22, 30].map(|offset| {
                    i16::from_le_bytes([oam[base + offset], oam[base + offset + 1]]) as i32
                })
            }),
        })
    }

    /// OBJ mosaic repeats samples on the screen grid, clamping groups that begin
    /// before the object to its first source texel,
    /// before flips or affine transforms; window selection uses the destination pixel.
    fn mosaic(
        &self,
        local_x: usize,
        local_y: usize,
        screen_x: usize,
        screen_y: usize,
        io: &[u8],
    ) -> (usize, usize) {
        if self.attr0 & 0x1000 == 0 {
            return (local_x, local_y);
        }

        let width = usize::from(io[0x4d] & 0x0f) + 1;
        let height = usize::from(io[0x4d] >> 4) + 1;

        (
            local_x.saturating_sub(screen_x % width),
            local_y.saturating_sub(screen_y % height),
        )
    }

    /// Finishes the final screen-aligned horizontal mosaic block. Both visible
    /// objects and object windows use this coverage; nominal bounds still define
    /// source geometry and vertical clipping. Signed X preserves left-edge wrapping.
    fn horizontal_coverage(&self, io: &[u8]) -> usize {
        let (width, _) = self.bounds();
        if self.attr0 & 0x1000 == 0 {
            return width;
        }
        let mosaic_width = i32::from(io[0x4d] & 15) + 1;
        let raw_x = i32::from(self.attr1 & 511);
        let x = if raw_x >= 256 { raw_x - 512 } else { raw_x };
        let end = x + width as i32;
        width + (mosaic_width - end.rem_euclid(mosaic_width)).rem_euclid(mosaic_width) as usize
    }

    /// Expanded bounds change the center and clipping rectangle, never source stride.
    fn bounds(&self) -> (usize, usize) {
        let factor = if self.matrix.is_some() && self.attr0 & 0x200 != 0 {
            2
        } else {
            1
        };
        (self.width * factor, self.height * factor)
    }

    /// Resolves one local texel through OBJ character memory and OBJ palette RAM.
    /// Tile numbers count 32-byte units in both color depths and wrap in 32 KiB.
    fn pixel(
        &self,
        mut x: usize,
        mut y: usize,
        control: u16,
        vram: &[u8],
        palette: &[u8],
    ) -> Option<u16> {
        if let Some([pa, pb, pc, pd]) = self.matrix {
            let (bound_width, bound_height) = self.bounds();
            let dx = x as i32 - bound_width as i32 / 2;
            let dy = y as i32 - bound_height as i32 / 2;
            // Arithmetic shifts retain hardware flooring for negative fractions.
            // OAM supplies a screen-to-source matrix, including singular matrices.
            let source_x = ((pa * dx + pb * dy) >> 8) + self.width as i32 / 2;
            let source_y = ((pc * dx + pd * dy) >> 8) + self.height as i32 / 2;
            if source_x < 0
                || source_y < 0
                || source_x >= self.width as i32
                || source_y >= self.height as i32
            {
                return None;
            }
            x = source_x as usize;
            y = source_y as usize;
        } else {
            if self.attr1 & (1 << 12) != 0 {
                x = self.width - 1 - x;
            }
            if self.attr1 & (1 << 13) != 0 {
                y = self.height - 1 - y;
            }
        }
        let color_256 = self.attr0 & (1 << 13) != 0;
        let units = if color_256 { 2 } else { 1 };
        let one_dimensional = control & (1 << 6) != 0;

        let mut base = usize::from(self.attr2 & 0x03ff);

        let tile_x = x / 8;
        let tile_y = y / 8;

        let tile = if one_dimensional {
            // 1D OBJ data is contiguous. Character names are 32-byte units,
            // including the guest-supplied low bit in 8bpp mode.
            let row_stride = (self.width / 8) * units;

            (base + tile_y * row_stride + tile_x * units) & 0x03ff
        } else {
            // In 8bpp 2D mapping, character-name bit 0 is ignored.
            if color_256 {
                base &= !1;
            }

            // 2D mapping is a 32-unit-wide character matrix. Horizontal addressing
            // wraps inside the current row rather than carrying into the next row.
            let row = (base & !31) + tile_y * 32;
            let column = ((base & 31) + tile_x * units) & 31;

            (row + column) & 0x03ff
        };

        let offset = 0x10000 + tile * 32;
        let (color, bank) = if color_256 {
            (usize::from(vram[offset + (y % 8) * 8 + x % 8]), 0)
        } else {
            let packed = vram[offset + (y % 8) * 4 + x % 8 / 2];
            (
                usize::from((packed >> ((x % 2) * 4)) & 15),
                usize::from(self.attr2 >> 12) * 16,
            )
        };
        if color == 0 {
            return None;
        }
        let offset = 0x200 + (bank + color) * 2;
        Some(u16::from_le_bytes([palette[offset], palette[offset + 1]]))
    }
}

/// Affine source sampling uses signed 8.8 coefficients and signed 28-bit origins.
/// The display owns accumulated origins; this snapshot resolves one scanline.
struct AffineBackground {
    control: u16,
    origin: [i32; 2],
    step: [i32; 2],
}

impl AffineBackground {
    /// Captures horizontal coefficients while retaining the display's current line origin.
    fn new(io: &[u8], background: usize, origin: [i32; 2]) -> Self {
        let base = 0x20 + (background - 2) * 16;
        Self {
            control: u16::from_le_bytes([io[8 + background * 2], io[9 + background * 2]]),
            origin,
            step: [0, 4].map(|offset| {
                i16::from_le_bytes([io[base + offset], io[base + offset + 1]]) as i32
            }),
        }
    }

    /// Bitmap bounds always clip; tiled backgrounds may wrap through BGxCNT.
    fn pixel(
        &self,
        screen_x: usize,
        mode: u16,
        page: usize,
        vram: &[u8],
        palette: &[u8],
    ) -> Option<u16> {
        let [mut x, mut y] = std::array::from_fn(|axis| {
            self.origin[axis].wrapping_add(self.step[axis] * screen_x as i32) >> 8
        });
        let (width, height) = if mode == 5 {
            (160, 128)
        } else if mode >= 3 {
            (240, 160)
        } else {
            let size = 128 << (self.control >> 14);
            (size, size)
        };
        if mode < 3 && self.control & 0x2000 != 0 {
            x = x.rem_euclid(width);
            y = y.rem_euclid(height);
        } else if x < 0 || y < 0 || x >= width || y >= height {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        if mode == 3 || mode == 5 {
            let offset = (if mode == 5 { page } else { 0 }) + (y * width as usize + x) * 2;
            return Some(u16::from_le_bytes([vram[offset], vram[offset + 1]]));
        }
        let color = if mode == 4 {
            vram[page + y * 240 + x]
        } else {
            let map = usize::from((self.control >> 8) & 31) * 0x800;
            let tile = usize::from(vram[(map + y / 8 * (width as usize / 8) + x / 8) & 0xffff]);
            let base = usize::from((self.control >> 2) & 3) * 0x4000;
            vram[(base + tile * 64 + y % 8 * 8 + x % 8) & 0xffff]
        };
        if color == 0 {
            return None;
        }
        let offset = usize::from(color) * 2;
        Some(u16::from_le_bytes([palette[offset], palette[offset + 1]]))
    }
}

/// Window selection precedes layer priority. Bits 0..4 enable BG0..3/OBJ;
/// bit 5 preserves permission for the subsequent color-effects stage.
struct WindowMasks {
    layers: [u8; SCREEN_WIDTH],
}

impl WindowMasks {
    /// Hardware ranges are half-open and wrap when the start exceeds the end.
    fn contains(bounds: u16, coordinate: usize) -> bool {
        let start = usize::from(bounds >> 8);
        let end = usize::from(bounds & 255);
        if start <= end {
            (start..end).contains(&coordinate)
        } else {
            coordinate >= start || coordinate < end
        }
    }

    /// WIN0 overrides WIN1, which overrides OBJWIN, which overrides WINOUT.
    /// Transparent OBJWIN texels leave the previous outside mask intact.
    fn scanline(
        control: u16,
        line: usize,
        io: &[u8],
        oam: &[u8],
        vram: &[u8],
        palette: &[u8],
    ) -> Self {
        let mut layers = [0x3f; SCREEN_WIDTH];
        if control & 0xe000 == 0 {
            return Self { layers };
        }
        layers.fill(io[0x4a] & 0x3f);
        if control & 0x9000 == 0x9000 {
            for entry in oam.as_chunks::<8>().0 {
                let Some(object) = Object::from_oam(entry, oam) else {
                    continue;
                };
                if object.attr0 & 0x0c00 != 0x0800 {
                    continue;
                }
                let y = (line + 256 - usize::from(object.attr0 & 255)) & 255;
                let (_, height) = object.bounds();
                if y >= height {
                    continue;
                }
                for local_x in 0..object.horizontal_coverage(io) {
                    let x = (usize::from(object.attr1 & 511) + local_x) & 511;
                    if x >= SCREEN_WIDTH {
                        continue;
                    }

                    let (sample_x, sample_y) = object.mosaic(local_x, y, x, line, io);

                    if object
                        .pixel(sample_x, sample_y, control, vram, palette)
                        .is_some()
                    {
                        layers[x] = io[0x4b] & 0x3f;
                    }
                }
            }
        }
        for window in (0..2).rev() {
            let horizontal_offset = 0x40 + window * 2;
            let vertical_offset = 0x44 + window * 2;

            let horizontal = u16::from_le_bytes([io[horizontal_offset], io[horizontal_offset + 1]]);

            let vertical = u16::from_le_bytes([io[vertical_offset], io[vertical_offset + 1]]);

            if control & (0x2000 << window) == 0 || !Self::contains(vertical, line) {
                continue;
            }

            let x_start = usize::from(horizontal >> 8);
            let x_end = usize::from(horizontal & 0xff);
            let window_mask = io[0x48 + window] & 0x3f;

            for (x, mask) in layers.iter_mut().enumerate() {
                let inside = if x_start <= x_end {
                    x >= x_start && x < x_end
                } else {
                    x >= x_start || x < x_end
                };

                if inside {
                    *mask = window_mask;
                }
            }
        }
        Self { layers }
    }
}

const BLEND_LAYER_OBJ: u8 = 4;
const BLEND_LAYER_BACKDROP: u8 = 5;

/// One opaque surface after layer/window filtering but before color effects.
///
/// `color` intentionally retains RGB555 bit 15 until the final mixer because
/// GBA blend hardware exposes that bit as an additional green precision bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayerPixel {
    color: u16,
    layer: u8,
    priority: u8,
    semitransparent: bool,
}

impl LayerPixel {
    #[inline]
    fn backdrop(color: u16) -> Self {
        Self {
            color,
            layer: BLEND_LAYER_BACKDROP,
            priority: 0,
            semitransparent: false,
        }
    }

    /// Lower values are visually closer to the viewer.
    ///
    /// At equal BG/OBJ priority the OBJ wins. Among equal-priority BGs,
    /// lower BG number wins. The backdrop is always below ordinary layers.
    #[inline]
    fn order(self) -> u8 {
        match self.layer {
            BLEND_LAYER_OBJ => self.priority * 5,
            0..=3 => self.priority * 5 + self.layer + 1,
            BLEND_LAYER_BACKDROP => 20,
            _ => u8::MAX,
        }
    }
}

/// Only the first two visible surfaces matter to the GBA color-effects stage.
///
/// Keeping exactly two is intentional: alpha blending may not skip a visible
/// non-second-target surface in order to find a deeper eligible target.
#[derive(Clone, Copy, Debug)]
struct PixelStack {
    top: LayerPixel,
    second: Option<LayerPixel>,
}

impl PixelStack {
    #[inline]
    fn new(backdrop: u16) -> Self {
        Self {
            top: LayerPixel::backdrop(backdrop),
            second: None,
        }
    }

    #[inline]
    fn insert(&mut self, pixel: LayerPixel) {
        let order = pixel.order();
        let top_order = self.top.order();

        if order < top_order {
            self.second = Some(self.top);
            self.top = pixel;
            return;
        }

        if order > top_order && self.second.is_none_or(|second| order < second.order()) {
            self.second = Some(pixel);
        }
    }
}

/// Snapshot of BLDCNT/BLDALPHA/BLDY at one drawing boundary.
#[derive(Clone, Copy, Debug)]
struct ColorEffects {
    first: u8,
    second: u8,
    mode: u8,
    eva: u8,
    evb: u8,
    evy: u8,
}

impl ColorEffects {
    fn from_io(io: &[u8]) -> Self {
        let read = |offset| u16::from_le_bytes([io[offset], io[offset + 1]]);

        let control = read(0x50);
        let alpha = read(0x52);
        let y = read(0x54);

        Self {
            first: (control & 0x003f) as u8,
            second: ((control >> 8) & 0x003f) as u8,
            mode: ((control >> 6) & 3) as u8,
            eva: ((alpha & 0x001f).min(16)) as u8,
            evb: (((alpha >> 8) & 0x001f).min(16)) as u8,
            evy: ((y & 0x001f).min(16)) as u8,
        }
    }

    #[inline]
    fn is_first_target(self, layer: u8) -> bool {
        self.first & (1u8 << layer) != 0
    }

    #[inline]
    fn is_second_target(self, layer: u8) -> bool {
        self.second & (1u8 << layer) != 0
    }

    /// Uses the hardware-observed extra green precision bit and rounds to nearest.
    #[inline]
    fn alpha_blend(first: u16, second: u16, eva: u8, evb: u8) -> u16 {
        let eva = u32::from(eva.min(16));
        let evb = u32::from(evb.min(16));

        let r1 = u32::from(first & 31);
        let g1 = u32::from(((first >> 4) & 62) | (first >> 15));
        let b1 = u32::from((first >> 10) & 31);

        let r2 = u32::from(second & 31);
        let g2 = u32::from(((second >> 4) & 62) | (second >> 15));
        let b2 = u32::from((second >> 10) & 31);

        let r = ((r1 * eva + r2 * evb + 8) >> 4).min(31);
        let g = ((g1 * eva + g2 * evb + 8) >> 4).min(63) >> 1;
        let b = ((b1 * eva + b2 * evb + 8) >> 4).min(31);

        (r | (g << 5) | (b << 10)) as u16
    }

    #[inline]
    fn brighten(color: u16, evy: u8) -> u16 {
        let evy = u32::from(evy.min(16));

        let mut r = u32::from(color & 31);
        let mut g = u32::from(((color >> 4) & 62) | (color >> 15));
        let mut b = u32::from((color >> 10) & 31);

        r += ((31 - r) * evy + 8) >> 4;
        g += ((63 - g) * evy + 8) >> 4;
        b += ((31 - b) * evy + 8) >> 4;

        (r | ((g >> 1) << 5) | (b << 10)) as u16 & 0x7fff
    }

    #[inline]
    fn darken(color: u16, evy: u8) -> u16 {
        let evy = u32::from(evy.min(16));

        let mut r = u32::from(color & 31);
        let mut g = u32::from(((color >> 4) & 62) | (color >> 15));
        let mut b = u32::from((color >> 10) & 31);

        r -= (r * evy + 7) >> 4;
        g -= (g * evy + 7) >> 4;
        b -= (b * evy + 7) >> 4;

        (r | ((g >> 1) << 5) | (b << 10)) as u16 & 0x7fff
    }

    #[inline]
    fn resolve(self, stack: PixelStack, effects_allowed: bool) -> u16 {
        let top = stack.top;

        /*
         * A semi-transparent OBJ forces alpha mode and is a first target
         * regardless of BLDCNT's OBJ-first bit.
         *
         * This forced alpha decision precedes the ordinary window SFX gate.
         * If the immediately lower surface is an enabled second target, alpha
         * wins even when BLDCNT selected brighten/darken.
         */
        if top.layer == BLEND_LAYER_OBJ
            && top.semitransparent
            && let Some(second) = stack.second
            && self.is_second_target(second.layer)
        {
            return Self::alpha_blend(top.color, second.color, self.eva, self.evb);
        }

        // Ordinary alpha/brightness is controlled by the selected window's
        // special-effects bit and BLDCNT first-target selection.
        if !effects_allowed || !self.is_first_target(top.layer) {
            return top.color & 0x7fff;
        }

        match self.mode {
            // None
            0 => top.color & 0x7fff,

            // Alpha. The immediate lower visible surface must itself be an
            // eligible second target; never search further down the stack.
            1 => {
                if let Some(second) = stack.second
                    && self.is_second_target(second.layer)
                {
                    Self::alpha_blend(top.color, second.color, self.eva, self.evb)
                } else {
                    top.color & 0x7fff
                }
            }

            // Brighten
            2 => Self::brighten(top.color, self.evy),

            // Darken
            3 => Self::darken(top.color, self.evy),

            _ => top.color & 0x7fff,
        }
    }
}

/// Display timing is independent of VRAM writes and frontend presentation.
struct Display {
    control: u16,
    /// Internal BG2/BG3 origins advance independently of the visible MMIO latches.
    affine_origin: [[i32; 2]; 2],
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
            affine_origin: [[0; 2]; 2],
            drawing: vec![0; FRAMEBUFFER_PIXELS],
            completed: vec![0; FRAMEBUFFER_PIXELS],
            generation: 0,
            frame_start: 0,
            line: 0,
            next_event: 960,
        }
    }

    /// Reloads only the written coordinate, including partial byte/halfword stores.
    fn write_reference(&mut self, offset: usize, io: &[u8]) {
        if (0x28..0x30).contains(&offset) || (0x38..0x40).contains(&offset) {
            let background = usize::from(offset >= 0x38);
            let axis = (offset & 7) / 4;
            let base = 0x28 + background * 16 + axis * 4;
            let raw = i32::from_le_bytes([io[base], io[base + 1], io[base + 2], io[base + 3]]);
            self.affine_origin[background][axis] = (raw << 4) >> 4;
        }
    }

    /// Isolates the first opaque OBJ texel in OAM order before combining that
    /// single OBJ surface with backgrounds. A later OBJ cannot participate in
    /// OBJ-to-OBJ blending through the color-effects stage.
    fn render_objects(
        &self,
        vram: &[u8],
        palette: &[u8],
        oam: &[u8],
        masks: &WindowMasks,
        io: &[u8],
        stacks: &mut [PixelStack; SCREEN_WIDTH],
    ) {
        let mut objects: [Option<LayerPixel>; SCREEN_WIDTH] = [None; SCREEN_WIDTH];

        for entry in oam.as_chunks::<8>().0 {
            let Some(object) = Object::from_oam(entry, oam) else {
                continue;
            };

            let object_mode = object.attr0 & 0x0c00;

            // Object-window pixels affect WindowMasks but are not visible OBJ pixels.
            if object_mode == 0x0800 {
                continue;
            }

            let y = (self.line + 256 - usize::from(object.attr0 & 255)) & 255;
            let (_, bound_height) = object.bounds();

            if y >= bound_height {
                continue;
            }

            for local_x in 0..object.horizontal_coverage(io) {
                let x = (usize::from(object.attr1 & 511) + local_x) & 511;

                if x >= SCREEN_WIDTH || masks.layers[x] & 0x10 == 0 || objects[x].is_some() {
                    continue;
                }

                let (sample_x, sample_y) = object.mosaic(local_x, y, x, self.line, io);

                if let Some(color) = object.pixel(sample_x, sample_y, self.control, vram, palette) {
                    objects[x] = Some(LayerPixel {
                        color,
                        layer: BLEND_LAYER_OBJ,
                        priority: ((object.attr2 >> 10) & 3) as u8,
                        semitransparent: object_mode == 0x0400,
                    });
                }
            }
        }

        for (stack, object) in stacks.iter_mut().zip(objects) {
            if let Some(object) = object {
                stack.insert(object);
            }
        }
    }

    /// Renders each visible scanline at its drawing boundary and publishes at VBlank.
    /// Within-line register effects remain the documented scanline approximation.
    fn synchronize_to(&mut self, target: u64, vram: &[u8], palette: &[u8], io: &[u8], oam: &[u8]) {
        while self.next_event <= target {
            if self.line < SCREEN_HEIGHT {
                let mode = self.control & 7;

                if self.line == 0 {
                    for offset in [0x28, 0x2c, 0x38, 0x3c] {
                        self.write_reference(offset, io);
                    }
                }

                let page = if self.control & 0x10 != 0 { 0xa000 } else { 0 };

                let forced_blank = self.control & 0x80 != 0;
                let start = self.line * SCREEN_WIDTH;

                if forced_blank {
                    self.drawing[start..start + SCREEN_WIDTH].fill(0x7fff);
                } else {
                    // Preserve the raw palette bit 15 until color effects finish.
                    let backdrop = u16::from_le_bytes([palette[0], palette[1]]);

                    let masks =
                        WindowMasks::scanline(self.control, self.line, io, oam, vram, palette);

                    let mut stacks = [PixelStack::new(backdrop); SCREEN_WIDTH];

                    /*
                     * Produce visible BG surfaces independently of final priority
                     * composition. PixelStack retains exactly the two nearest
                     * nontransparent surfaces required by the effects hardware.
                     *
                     * MOSAIC's BG dimensions are scanline-global register state, so
                     * decode them once rather than once per pixel.
                     */
                    let mosaic_width = usize::from(io[0x4c] & 0x0f) + 1;
                    let mosaic_height = usize::from(io[0x4c] >> 4) + 1;

                    for priority in (0..4).rev() {
                        for background in (0..4).rev() {
                            let control_offset = 8 + background * 2;

                            let bg_control =
                                u16::from_le_bytes([io[control_offset], io[control_offset + 1]]);

                            if self.control & (1 << (8 + background)) == 0
                                || bg_control & 3 != u16::from(priority)
                            {
                                continue;
                            }

                            let text = mode == 0 || (mode == 1 && background < 2);

                            let affine = (mode == 1 && background == 2)
                                || (mode == 2 && background >= 2)
                                || ((3..=5).contains(&mode) && background == 2);

                            if !text && !affine {
                                continue;
                            }

                            let mosaic = bg_control & 0x40 != 0;

                            let sample_y = if mosaic {
                                self.line / mosaic_height * mosaic_height
                            } else {
                                self.line
                            };

                            if text {
                                /*
                                 * TextBackground contains only scanline-invariant register
                                 * state, so construct it once for the complete BG scanline.
                                 */
                                let text_layer = TextBackground::from_registers(io, background);

                                for (x, stack) in stacks.iter_mut().enumerate() {
                                    if masks.layers[x] & (1u8 << background) == 0 {
                                        continue;
                                    }

                                    let sample_x = if mosaic {
                                        x / mosaic_width * mosaic_width
                                    } else {
                                        x
                                    };

                                    if let Some(color) =
                                        text_layer.pixel(sample_x, sample_y, vram, palette)
                                    {
                                        stack.insert(LayerPixel {
                                            color,
                                            layer: background as u8,
                                            priority,
                                            semitransparent: false,
                                        });
                                    }
                                }
                            } else {
                                /*
                                 * Vertical affine deltas and the scanline's affine origin
                                 * are invariant across X. The old code rebuilt this
                                 * AffineBackground for every destination pixel.
                                 */
                                let origin = std::array::from_fn(|axis| {
                                    let offset = 0x22 + (background - 2) * 16 + axis * 4;

                                    let delta =
                                        i16::from_le_bytes([io[offset], io[offset + 1]]) as i32;

                                    /*
                                     * Vertical mosaic samples the first scanline of the
                                     * current mosaic group while the hardware affine
                                     * accumulator itself continues advancing each line.
                                     */
                                    self.affine_origin[background - 2][axis]
                                        .wrapping_sub(delta * (self.line - sample_y) as i32)
                                });

                                /*
                                 * PA/PC horizontal steps, BGCNT and the adjusted line origin
                                 * remain constant across all 240 pixels.
                                 */
                                let affine_layer = AffineBackground::new(io, background, origin);

                                for (x, stack) in stacks.iter_mut().enumerate() {
                                    if masks.layers[x] & (1u8 << background) == 0 {
                                        continue;
                                    }

                                    let sample_x = if mosaic {
                                        x / mosaic_width * mosaic_width
                                    } else {
                                        x
                                    };

                                    if let Some(color) =
                                        affine_layer.pixel(sample_x, mode, page, vram, palette)
                                    {
                                        stack.insert(LayerPixel {
                                            color,
                                            layer: background as u8,
                                            priority,
                                            semitransparent: false,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    if self.control & (1 << 12) != 0 {
                        self.render_objects(vram, palette, oam, &masks, io, &mut stacks);
                    }

                    let effects = ColorEffects::from_io(io);

                    for (x, stack) in stacks.into_iter().enumerate() {
                        self.drawing[start + x] =
                            effects.resolve(stack, masks.layers[x] & 0x20 != 0);
                    }
                }

                /*
                 * Affine internal reference points keep advancing even if this
                 * particular scanline was forced blank.
                 */
                for (background, origin) in self.affine_origin.iter_mut().enumerate() {
                    for (axis, coordinate) in origin.iter_mut().enumerate() {
                        let offset = 0x22 + background * 16 + axis * 4;
                        let delta = i16::from_le_bytes([io[offset], io[offset + 1]]) as i32;

                        *coordinate = (coordinate.wrapping_add(delta) << 4) >> 4;
                    }
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
    /// Reset retains the selected boot route; diagnostics remain controlled.
    bios_startup: bool,
    backup: BackupSelection,
    cpu: Cpu,
    system: System,
    executed_instructions: usize,
}

impl Machine {
    /// Applies a live host transition at the current instruction boundary.
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        if !self.system.buttons.set(button, pressed) {
            return;
        }

        self.system.evaluate_keypad();
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

    /// Validates firmware before changing ownership. Loading does not execute or
    /// reset controlled diagnostics. An active BIOS session reboots with backup
    /// storage retained, preventing prefetched instructions from the previous image.
    pub fn load_bios(&mut self, bytes: &[u8]) -> Result<(), CoreError> {
        if bytes.len() != BIOS_SIZE {
            return Err(CoreError::InvalidBiosSize { size: bytes.len() });
        }
        let mut bios = Box::new([0u8; BIOS_SIZE]);
        bios.copy_from_slice(bytes);

        self.system.bios = Some(bios);
        if self.bios_startup {
            self.reset();
        }
        Ok(())
    }

    /// Boots a cartridge from the hardware reset vector. No diagnostic firmware
    /// or controlled stack/status state participates in this route.
    pub fn boot_rom_with_bios(
        &mut self,
        rom: &[u8],
        manual_override: Option<BackupType>,
    ) -> Result<(), CoreError> {
        if self.system.bios.is_none() {
            return Err(CoreError::MissingBios);
        }
        self.load_rom_with_backup(rom, manual_override)?;
        self.bios_startup = true;
        self.start_bios();
        Ok(())
    }

    /// Restores reset-vector execution in ARM supervisor mode with IRQ/FIQ
    /// masked. Firmware owns stack initialization and every subsequent service.
    fn start_bios(&mut self) {
        self.system.io[0x20..0x40].fill(0);
        self.cpu = Cpu::new();
        self.cpu.registers = [0; 16];
        self.cpu.cpsr = CPSR_I | CPSR_F | 0x13;
        self.system.execution_address = 0;
        self.system.bios_latch = 0;
        self.system.bios_enabled = true;
        self.system.test_firmware = false;
    }

    /// Loads a byte-based cartridge image and resets execution to its entry point.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), CoreError> {
        self.load_rom_with_backup(rom, None)
    }

    /// Loads a cartridge with a typed, validated override; failed loads retain state.
    pub fn load_rom_with_backup(
        &mut self,
        rom: &[u8],
        manual_override: Option<BackupType>,
    ) -> Result<(), CoreError> {
        if rom.is_empty() {
            return Err(CoreError::EmptyRom);
        }
        if rom.len() > MAX_ROM_BYTES {
            return Err(CoreError::RomTooLarge {
                size: rom.len(),
                maximum: MAX_ROM_BYTES,
            });
        }

        let bios = self.system.bios.take();
        *self = Self::new();
        self.system.bios = bios;
        self.backup = BackupSelection {
            detection: detect_backup(rom),
            manual_override,
        };
        if self.backup.selected() == Some(BackupType::Sram) {
            self.system.sram = Some(sram::Sram::new());
        } else if matches!(
            self.backup.selected(),
            Some(BackupType::Flash64 | BackupType::Flash128)
        ) {
            self.system.flash = Some(flash::Flash::new(
                self.backup.selected() == Some(BackupType::Flash128),
            ));
        }
        if matches!(
            self.backup.selected(),
            Some(BackupType::Eeprom | BackupType::Eeprom512 | BackupType::Eeprom8k)
        ) {
            let capacity = match self.backup.selected() {
                Some(BackupType::Eeprom512) => Some(EEPROM512_BYTES),
                Some(BackupType::Eeprom8k) => Some(EEPROM8K_BYTES),
                _ => None,
            };
            self.system.eeprom = Some(eeprom::Eeprom::new(capacity));
        }
        self.system.rom.extend_from_slice(rom);
        Ok(())
    }

    /// Reports cartridge evidence and the effective save-hardware selection.
    pub fn backup_selection(&self) -> &BackupSelection {
        &self.backup
    }

    /// Copies cartridge bytes and their revision for asynchronous host storage.
    pub fn save_image(&self) -> Option<SaveImage> {
        self.system
            .sram
            .as_ref()
            .map(sram::Sram::image)
            .or_else(|| self.system.flash.as_ref().map(flash::Flash::image))
            .or_else(|| self.system.eeprom.as_ref().map(eeprom::Eeprom::image))
    }

    /// Reports cartridge save metadata without copying the owned backup bytes.
    pub fn save_status(&self) -> Option<SaveStatus> {
        self.system
            .sram
            .as_ref()
            .map(sram::Sram::status)
            .or_else(|| self.system.flash.as_ref().map(flash::Flash::status))
            .or_else(|| self.system.eeprom.as_ref().map(eeprom::Eeprom::status))
    }

    /// Loads validated initial bytes before execution, without making them dirty.
    pub fn load_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if let Some(sram) = &mut self.system.sram {
            sram.load(bytes)
        } else if let Some(flash) = &mut self.system.flash {
            flash.load(bytes)
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.load(bytes)
        } else {
            Err("cartridge has no supported backup hardware")
        }
    }

    /// Imports bytes as a newer revision, even when the content is unchanged.
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if let Some(sram) = &mut self.system.sram {
            sram.import(bytes)
        } else if let Some(flash) = &mut self.system.flash {
            flash.import(bytes)
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.import(bytes)
        } else {
            Err("cartridge has no supported backup hardware")
        }
    }

    /// A completion can acknowledge only the revision actually written.
    pub fn acknowledge_save(&mut self, revision: u64) {
        if let Some(sram) = &mut self.system.sram {
            sram.acknowledge(revision);
        } else if let Some(flash) = &mut self.system.flash {
            flash.acknowledge(revision);
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.acknowledge(revision);
        }
    }

    /// Enables the original SWI-division and IRQ-vector firmware for a controlled diagnostic
    /// session. Normal cartridge loading leaves the low firmware region unmapped.
    pub fn enable_test_firmware(&mut self) {
        if self.bios_startup {
            return;
        }
        self.system.test_firmware = true;
        self.cpu.exception_banks[1][0] = 0x0300_7fe0;
    }

    /// Drains completed mono PCM samples without advancing guest time.
    pub fn drain_pcm(&mut self, output: &mut Vec<f32>) {
        self.system.audio.drain(output);
    }

    /// Drains completed left/right PCM frames without advancing guest time.
    pub fn drain_stereo_pcm(&mut self, output: &mut Vec<[f32; 2]>) {
        self.system.audio.drain_stereo(output);
    }

    /// Discards host-bound samples at pause/focus boundaries, retaining devices.
    pub fn clear_pcm(&mut self) {
        self.system.audio.clear_pcm();
    }

    /// Reports produced, discarded and empty-FIFO sample counts since reset.
    pub fn pcm_counters(&self) -> (u64, u64, u64) {
        (
            self.system.audio.produced,
            self.system.audio.dropped,
            self.system.audio.fifo_underruns,
        )
    }

    /// Executes one instruction, one DMA read/write beat, or advances HALT to
    /// the next input/display event. DMA and sleep preserve instruction count.
    pub fn step(&mut self) -> Result<u32, CoreError> {
        if let Some(channel) = self.system.dma.iter().position(|dma| dma.active) {
            self.system.dma_beat(channel)?;
            return Ok(self.cpu.registers[15]);
        }
        let pending = self.system.pending_interrupts();
        if pending != 0 {
            self.system.halted = false;
            if self.system.io[0x208] & 1 != 0 && self.cpu.cpsr & CPSR_I == 0 {
                let status = self.cpu.cpsr;
                let resume = self.cpu.registers[15];
                self.cpu
                    .set_status((status & !(31 | CPSR_T)) | 0x12 | CPSR_I);
                self.cpu.saved_status[1] = status;
                self.cpu.registers[14] = resume.wrapping_add(4);
                // Delivery always fetches the mapped vector; no host callback shortcut.
                self.system.idle(1);
                self.cpu.branch(&mut self.system, 0x18, false)?;
            }
        }
        if self.system.halted {
            // Bound a public step to the next input/display event. advance_to additionally
            // clips sleeping time to its caller's absolute deadline.
            let next = self.system.next_wake_event();
            self.system.advance_time(next - self.system.cycles);
            return Ok(self.cpu.registers[15]);
        }
        let outcome = self.cpu.step(&mut self.system)?;
        self.executed_instructions = self.executed_instructions.saturating_add(1);
        Ok(outcome.address)
    }

    /// Advances toward an absolute cycle deadline with a hard instruction budget.
    /// An instruction or DMA beat may finish beyond the deadline; subsequent
    /// targets retain that excess. A complete DMA descriptor never runs atomically.
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
            if self.system.halted
                && self.system.pending_interrupts() == 0
                && !self.system.dma.iter().any(|dma| dma.active)
            {
                let next = self.system.next_wake_event();
                let deadline = next.min(target.0);
                self.system.advance_time(deadline - self.system.cycles);
                continue;
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
            let before = self.executed_instructions;
            let pc = self.step()?;
            if self.cycles() > cycle_limit {
                return Err(RunError::CycleLimitExceeded {
                    limit: cycle_limit,
                    reached: self.cycles(),
                    last_pc: pc,
                });
            }
            if pc == terminal_pc && self.executed_instructions != before {
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

    /// Reports hardware sleep for bounded fixture stall checks and application status.
    pub fn halted(&self) -> bool {
        self.system.halted
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

    /// Exposes architectural registers for manifest-bound diagnostic results.
    pub fn registers(&self) -> &[u32; 16] {
        &self.cpu.registers
    }

    /// Exposes status for diagnostic completion and exception-return checks.
    pub fn cpsr(&self) -> u32 {
        self.cpu.cpsr
    }

    /// Reports the active instruction set for bounded guest-return diagnostics.
    pub fn is_thumb(&self) -> bool {
        self.cpu.cpsr & CPSR_T != 0
    }

    /// Returns the number of successfully executed guest instructions since reset.
    pub fn executed_instructions(&self) -> usize {
        self.executed_instructions
    }

    /// Resets CPU, memory, and display state while retaining the loaded ROM bytes.
    pub fn reset(&mut self) {
        let rom = std::mem::take(&mut self.system.rom);
        let firmware = self.system.test_firmware;
        let bios = self.system.bios.take();
        let bios_startup = self.bios_startup;
        let backup = std::mem::take(&mut self.backup);
        let sram = self.system.sram.take();
        let mut eeprom = self.system.eeprom.take();
        if let Some(chip) = &mut eeprom {
            chip.reset();
        }
        let mut flash = self.system.flash.take();
        if let Some(chip) = &mut flash {
            chip.reset();
        }
        *self = Self::new();
        self.system.rom = rom;
        self.system.bios = bios;
        self.bios_startup = bios_startup;
        if bios_startup {
            self.start_bios();
        }
        self.backup = backup;
        self.system.sram = sram;
        self.system.flash = flash;
        self.system.eeprom = eeprom;
        if firmware {
            self.enable_test_firmware();
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            bios_startup: false,
            backup: BackupSelection::default(),
            cpu: Cpu::new(),
            system: System::new(),
            executed_instructions: 0,
        }
    }
}

/// Ordinary stores align to their transfer width. The cartridge backup bus
/// instead needs the original low address bits to select one source byte.
fn store_address(address: u32, alignment_mask: u32) -> u32 {
    if (0x0e000000..0x10000000).contains(&address) {
        address
    } else {
        address & !alignment_mask
    }
}

/// VRAM repeats every 128 KiB; the last 32 KiB mirrors physical 64..96 KiB.
/// Decode before byte-store rules so mirrored OBJ addresses are also ignored.
fn vram_range(address: u32, width: usize) -> Option<Range<usize>> {
    if address >> 24 != VRAM_START >> 24 {
        return None;
    }
    let mut offset = address as usize & 0x1ffff;
    if offset >= 0x18000 {
        offset -= 0x8000;
    }
    (offset + width <= VRAM_BYTES).then_some(offset..offset + width)
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

    /// Catches truncating an unaligned OBJ's final mosaic block and anchoring
    /// vertical groups to the object. Expected pixels follow mGBA's OBJ renderer.
    #[test]
    fn unaligned_object_mosaic_repeats_through_right_edge() {
        let mut bus = System::new();
        bus.display.control = 0x1040;
        for entry in bus.oam.as_chunks_mut::<8>().0 {
            entry[..2].copy_from_slice(&0x0200u16.to_le_bytes());
        }
        bus.oam[..2].copy_from_slice(&0x103du16.to_le_bytes()); // Y=61, mosaic.
        bus.oam[2..4].copy_from_slice(&101u16.to_le_bytes());
        bus.io[0x4d] = 0x22; // Three screen pixels in each axis.
        for row in 0..8 {
            bus.vram[0x10000 + row * 4..0x10004 + row * 4].fill((row as u8 + 1) * 0x11);
            bus.palette[0x202 + row * 2..0x204 + row * 2]
                .copy_from_slice(&(row as u16 + 1).to_le_bytes());
        }
        let masks = WindowMasks {
            layers: [0x3f; SCREEN_WIDTH],
        };
        for (line, color) in [(61, 1), (62, 1), (63, 3)] {
            bus.display.line = line;
            let mut stacks = [PixelStack::new(0); SCREEN_WIDTH];
            bus.display.render_objects(
                &bus.vram,
                &bus.palette,
                &bus.oam,
                &masks,
                &bus.io,
                &mut stacks,
            );
            let row: [u16; SCREEN_WIDTH] = std::array::from_fn(|x| stacks[x].top.color);
            assert_eq!(row[100], 0);
            assert!(
                row[101..111].iter().all(|pixel| *pixel == color),
                "line {line}: right-edge repetition or vertical phase is wrong: {:?}",
                &row[101..112]
            );
            assert_eq!(row[111], 0);
        }
    }

    /// Catches recomputing origins from line number after a mid-frame write,
    /// and treating signed 28-bit reference coordinates as unsigned values.
    #[test]
    fn affine_reference_write_reloads_current_line_and_advances() {
        let mut bus = System::new();
        bus.display.control = 0x0403;
        bus.vram[0..2].copy_from_slice(&31u16.to_le_bytes());
        bus.vram[480..482].copy_from_slice(&992u16.to_le_bytes());
        bus.vram[960..962].copy_from_slice(&0x4210u16.to_le_bytes());
        bus.display
            .synchronize_to(960, &bus.vram, &bus.palette, &bus.io, &bus.oam);
        bus.write_halfword(0x0400002c, 0).unwrap();
        bus.write_halfword(0x0400002e, 0).unwrap();
        bus.display
            .synchronize_to(2192, &bus.vram, &bus.palette, &bus.io, &bus.oam);
        assert_eq!(bus.display.drawing[240], 31);
        bus.display
            .synchronize_to(3424, &bus.vram, &bus.palette, &bus.io, &bus.oam);
        assert_eq!(bus.display.drawing[480], 992);
        bus.write_halfword(0x04000028, 0xff00).unwrap();
        bus.write_halfword(0x0400002a, 0x0fff).unwrap();
        bus.display
            .synchronize_to(4656, &bus.vram, &bus.palette, &bus.io, &bus.oam);
        assert_eq!(bus.display.drawing[720], 0);
        assert_eq!(bus.display.drawing[721], 0x4210); // X=-1 clips, then X=0 samples row 2.
    }

    /// Catches screen-linear sampling, missing mode-5 pages, and incorrect clipping.
    #[test]
    fn affine_bitmap_rotation_clips_and_selects_page() {
        let mut display = Display::new();
        let mut io = vec![0; IO_BYTES];
        let mut vram = vec![0; VRAM_BYTES];
        let palette = vec![0; PALETTE_BYTES];
        let oam = vec![0; OAM_BYTES];
        io[0x24..0x26].copy_from_slice(&256i16.to_le_bytes());
        io[0x28..0x2c].copy_from_slice(&256i32.to_le_bytes());
        vram[0xa002..0xa004].copy_from_slice(&0x1234u16.to_le_bytes());
        vram[0xa142..0xa144].copy_from_slice(&0x5678u16.to_le_bytes());
        display.control = 0x0415;
        display.synchronize_to(960, &vram, &palette, &io, &oam);
        assert_eq!(display.drawing[0], 0x1234);
        assert_eq!(display.drawing[1], 0x5678);
        assert_eq!(display.drawing[128], 0);
    }

    #[test]
    fn retail_bios_undocumented_0410_byte_write_is_accepted() {
        let mut bus = System::new();
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        let before = bus.cycles;

        bus.write8(BIOS_UNDOCUMENTED_IO_410, 0xff, data)
            .expect("retail BIOS 0x04000410 byte write must be accepted");

        // The write still occupies a real bus access.
        assert!(bus.cycles > before);

        // Keep neighboring unused addresses outside the ordinary I/O bank.
        assert!(matches!(
            bus.write8(BIOS_UNDOCUMENTED_IO_410 + 1, 0xff, data),
            Err(CoreError::UnmappedAddress { address, width: 1 })
                if address == BIOS_UNDOCUMENTED_IO_410 + 1
        ));
    }

    /// DMA command lengths select the EEPROM address width. A high block must
    /// survive serial readback and restore without aliasing a smaller device.
    #[test]
    fn eeprom_dma_detects_capacity_and_restores_serial_blocks() {
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        for (address_bits, block, capacity) in [(6, 63, 512), (14, 1023, 8192)] {
            let mut machine = Machine::new();
            machine
                .load_rom(include_bytes!("../../../roms/backup/eeprom.gba"))
                .unwrap();
            let data = 0xa501_2345_6789_abcd_u64;
            let transfer = |machine: &mut Machine, bits: &[u16]| {
                for (index, bit) in bits.iter().enumerate() {
                    machine
                        .system
                        .write16_impl(0x02000000 + index as u32 * 2, *bit, access)
                        .unwrap();
                }
                for (address, value) in [(0x040000d4, 0x02000000), (0x040000d8, 0x0d000000)] {
                    machine.system.write32_impl(address, value, access).unwrap();
                }
                machine
                    .system
                    .write16_impl(0x040000dc, bits.len() as u16, access)
                    .unwrap();
                machine
                    .system
                    .write16_impl(0x040000de, 0x8000, access)
                    .unwrap();
                while machine.system.dma[3].active {
                    machine.system.dma_beat(3).unwrap();
                }
            };
            let address: Vec<u16> = (0..address_bits)
                .rev()
                .map(|shift| ((block >> shift) & 1) as u16)
                .collect();
            let mut write = vec![1, 0];
            write.extend_from_slice(&address);
            write.extend((0..64).rev().map(|shift| ((data >> shift) & 1) as u16));
            write.push(0);
            transfer(&mut machine, &write);
            let saved = machine
                .save_image()
                .expect("EEPROM must expose persistence");
            assert_eq!(saved.bytes.len(), capacity);
            assert_eq!(&saved.bytes[block * 8..block * 8 + 8], &data.to_be_bytes());
            assert!(saved.dirty);
            machine.load_save(&saved.bytes).unwrap();
            machine.reset();
            let mut read = vec![1, 1];
            read.extend_from_slice(&address);
            read.push(0);
            transfer(&mut machine, &read);
            for _ in 0..4 {
                machine.system.read16_impl(0x0d000000, access).unwrap();
            }
            let mut actual = 0_u64;
            for _ in 0..64 {
                actual = (actual << 1)
                    | u64::from(machine.system.read16_impl(0x0d000000, access).unwrap() & 1);
            }
            assert_eq!(actual, data);
            assert!(!machine.save_image().unwrap().dirty);
            assert!(
                machine
                    .import_save(&vec![0; if capacity == 512 { 8192 } else { 512 }])
                    .is_err()
            );
            assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
        }
    }

    /// Bank-local programming and sector erase must preserve the other bank.
    /// A complete persisted image restores both banks after protocol reset.
    #[test]
    fn flash_banks_preserve_and_restore_independent_bytes() {
        let mut machine = Machine::new();
        machine
            .load_rom(include_bytes!("../../../roms/backup/flash128.gba"))
            .unwrap();
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        let mut command = |bank, value| {
            for (offset, byte) in [
                (0x5555, 0xaa),
                (0x2aaa, 0x55),
                (0x5555, 0xb0),
                (0, bank),
                (0x5555, 0xaa),
                (0x2aaa, 0x55),
                (0x5555, 0xa0),
                (0x123, value),
            ] {
                machine
                    .system
                    .write8(0x0e000000 + offset, byte, access)
                    .unwrap();
            }
        };
        command(0, 0x42);
        command(1, 0x24);
        let saved = machine
            .save_image()
            .expect("Flash128 must expose persistence");
        assert_eq!(saved.bytes.len(), 131072);
        assert_eq!(saved.bytes[0x123], 0x42);
        assert_eq!(saved.bytes[0x10123], 0x24);
        for (offset, byte) in [
            (0x5555, 0xaa),
            (0x2aaa, 0x55),
            (0x5555, 0x80),
            (0x5555, 0xaa),
            (0x2aaa, 0x55),
            (0, 0x30),
        ] {
            machine
                .system
                .write8(0x0e000000 + offset, byte, access)
                .unwrap();
        }
        let erased = machine.save_image().unwrap();
        assert_eq!(erased.bytes[0x123], 0x42);
        assert_eq!(erased.bytes[0x10123], 0xff);
        machine.load_save(&saved.bytes).unwrap();
        machine.reset();
        assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
        assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
        assert!(machine.import_save(&saved.bytes[..65536]).is_err());
        assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
    }

    /// A save read interrupts an unfinished Flash unlock. Resuming its remaining
    /// writes must not program a byte; a fresh, uninterrupted sequence must work.
    #[test]
    fn flash_read_cancels_partial_unlock() {
        let mut machine = Machine::new();
        machine
            .load_rom(include_bytes!("../../../roms/backup/flash64.gba"))
            .unwrap();
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine.system.write8(0x0e005555, 0xaa, access).unwrap();
        assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0xff);
        for (offset, value) in [(0x2aaa, 0x55), (0x5555, 0xa0), (0x123, 0x42)] {
            machine
                .system
                .write8(0x0e000000 + offset, value, access)
                .unwrap();
        }
        assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0xff);
        assert!(!machine.save_image().unwrap().dirty);
        for (offset, value) in [
            (0x5555, 0xaa),
            (0x2aaa, 0x55),
            (0x5555, 0xa0),
            (0x123, 0x42),
        ] {
            machine
                .system
                .write8(0x0e000000 + offset, value, access)
                .unwrap();
        }
        assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
        machine.reset();
        assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
        assert!(machine.save_image().unwrap().dirty);
    }

    /// A detected SRAM cartridge must expose mirrored byte storage rather than
    /// returning unmapped-bus errors or losing the score across a CPU reset.
    #[test]
    fn sram_byte_bus_survives_reset() {
        let mut machine = Machine::new();
        machine
            .load_rom(include_bytes!("../../../roms/backup/sram.gba"))
            .unwrap();
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        assert_eq!(machine.system.read8(0x0e000000, access).unwrap(), 0xff);
        machine.system.write8(0x0e000003, 42, access).unwrap();
        assert_eq!(machine.system.read8(0x0f008003, access).unwrap(), 42);
        machine.reset();
        assert_eq!(machine.system.read8(0x0e000003, access).unwrap(), 42);
    }

    // A block load crossing from mapped MMIO into an unmapped address must
    // retain CPU pipeline context, rather than expose the preceding MMIO value.
    // Existing timing tests do not cross a mapped/unmapped boundary in one LDM.
    #[test]
    fn open_bus_block_load_retains_cpu_prefetch() {
        let words = [0xe8900006u32, 0xe1a00000, 0xe1a04004, 0xeafffffe];
        let rom: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        let mut machine = Machine::new();
        machine.load_rom(&rom).unwrap();
        machine.cpu.registers[0] = IO_START + 0x3fc;
        machine.step().unwrap();
        assert_eq!(machine.registers()[1], 0);
        assert_eq!(machine.registers()[2], 0xe1a04004);
    }

    // Existing controlled fixtures cannot catch commercial boot bypassing the
    // reset vector or losing cartridge storage when the BIOS is retained.
    #[test]
    fn supplied_bios_boot_and_reset_preserve_backup() {
        let mut machine = Machine::new();
        assert!(
            machine
                .boot_rom_with_bios(&[0; 8], Some(BackupType::Sram))
                .is_err()
        );
        assert!(machine.load_bios(&[0; 4]).is_err());
        let mut bios = vec![0; BIOS_SIZE];
        bios[..4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
        machine.load_bios(&bios).unwrap();
        machine
            .boot_rom_with_bios(&[0; 8], Some(BackupType::Sram))
            .unwrap();
        assert_eq!(machine.registers()[15], 0);
        assert_eq!(machine.registers()[13], 0);
        assert_eq!(machine.cpsr(), 0xd3);
        machine.system.backup_write(0x0e000000, 0x42);
        let saved = machine.save_image().unwrap();
        machine.step().unwrap();
        machine.reset();
        assert_eq!(machine.registers()[15], 0);
        assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
        assert_eq!(machine.save_image().unwrap().revision, saved.revision);
        machine.step().unwrap();
        machine.load_rom(&[0; 8]).unwrap();
        assert_eq!(machine.registers()[15], ROM_START);
    }

    // Protected reads must retain the last fetched BIOS word after a redirect;
    // exposing raw BIOS data would fail upstream startup/SWI/IRQ checks.
    #[test]
    fn bios_protection_uses_execution_context_and_fetch_latch() {
        let mut machine = Machine::new();
        let mut bios = vec![0; BIOS_SIZE];
        bios[..4].copy_from_slice(&0x12345678u32.to_le_bytes());
        bios[4..8].copy_from_slice(&0xabcdef01u32.to_le_bytes());
        machine.load_bios(&bios).unwrap();
        machine.system.bios_enabled = true;
        machine.system.execution_address = 0;
        let fetch = Access {
            kind: AccessKind::Fetch,
            sequential: false,
        };
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        assert_eq!(machine.system.read32(4, fetch).unwrap(), 0xabcdef01);
        assert_eq!(machine.system.read32(0, data).unwrap(), 0x12345678);
        machine.system.execution_address = ROM_START;
        assert_eq!(machine.system.read32(0, data).unwrap(), 0xabcdef01);
        assert_eq!(machine.system.read16(2, data).unwrap(), 0xabcd);
        assert_eq!(machine.system.read8(1, data).unwrap(), 0xef);
    }

    /// Exercises sleep, firmware dispatch, W1C and masked wake through guest code.
    #[test]
    fn vblank_guest_sleeps_dispatches_and_acknowledges() {
        let rom = include_bytes!("../../../roms/vblank.gba");
        let mut machine = Machine::new();
        machine.load_rom(rom).unwrap();
        machine.enable_test_firmware();
        machine
            .advance_to(Cycle(4 * CYCLES_PER_FRAME), 20_000)
            .unwrap();
        assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 4);
        assert_eq!(machine.inspect16(IWRAM_START + 4).unwrap(), 1);
        assert_eq!(machine.inspect16(IWRAM_START + 6).unwrap(), 0);
        assert_eq!(machine.inspect16(IWRAM_START + 8).unwrap(), 4);
        assert_eq!(machine.cpsr() & 0xff, 0x5f);
        assert!(machine.executed_instructions() < 1000);
        let mut thumb = rom.to_vec();
        thumb[0x300..0x304].copy_from_slice(&15u32.to_le_bytes());
        machine.load_rom(&thumb).unwrap();
        machine.enable_test_firmware();
        machine
            .advance_to(Cycle(4 * CYCLES_PER_FRAME), 20_000)
            .unwrap();
        assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 4);
        assert!(machine.is_thumb());
        for (configuration, wakes) in [(6u32, 0), (5, 2), (3, 2)] {
            let mut bytes = rom.to_vec();
            bytes[0x300..0x304].copy_from_slice(&configuration.to_le_bytes());
            machine.load_rom(&bytes).unwrap();
            machine.enable_test_firmware();
            machine
                .advance_to(Cycle(2 * CYCLES_PER_FRAME), 20_000)
                .unwrap();
            assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 0);
            assert_eq!(machine.inspect16(IWRAM_START + 8).unwrap(), wakes);
        }
        machine.load_rom(rom).unwrap();
        assert!(matches!(
            machine.advance_to(Cycle(CYCLES_PER_FRAME), 20_000),
            Err(RunError::Core(CoreError::UnmappedAddress {
                address: 0x18,
                width: 4
            }))
        ));
        assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 0);
    }

    // Catches ignored WAITCNT writes and ROM aliases charged as ordinary memory.
    // Existing fixtures use default WS0 timing exclusively.
    #[test]
    fn cartridge_windows_use_programmed_waitstates() {
        let mut bus = System::new();
        bus.rom = vec![0x34, 0x12, 0x78, 0x56];
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        bus.rom.resize(0x20004, 0);
        for (bank, address) in [0x08000000, 0x0a000000, 0x0c000000].into_iter().enumerate() {
            for (selector, first) in [5, 4, 3, 9].into_iter().enumerate() {
                for (fast, second) in [[3, 5, 9][bank], 2].into_iter().enumerate() {
                    let control = ((selector as u16) << [2, 5, 8][bank])
                        | ((fast as u16) << [4, 7, 10][bank]);
                    bus.write16_impl(IO_START + 0x204, control, data).unwrap();
                    let start = bus.cycles;
                    assert_eq!(bus.read32_impl(address, data).unwrap(), 0x56781234);
                    assert_eq!(bus.cycles - start, first + second);
                    let start = bus.cycles;
                    bus.read16_impl(
                        address + 4,
                        Access {
                            sequential: true,
                            ..data
                        },
                    )
                    .unwrap();
                    assert_eq!(bus.cycles - start, second);
                    // The cartridge forces N timing at each 128 KiB boundary.
                    bus.read16_impl(address + 0x1fffe, data).unwrap();
                    let start = bus.cycles;
                    bus.read16_impl(
                        address + 0x20000,
                        Access {
                            sequential: true,
                            ..data
                        },
                    )
                    .unwrap();
                    assert_eq!(bus.cycles - start, first);
                }
            }
        }
        for (selector, cycles) in [5, 4, 3, 9].into_iter().enumerate() {
            bus.write16_impl(IO_START + 0x204, selector as u16, data)
                .unwrap();
            let start = bus.cycles;
            bus.charge(0x0e000000, 1, data);
            assert_eq!(bus.cycles - start, cycles);
        }
    }

    // Catches lost partial fills, incorrect word consumption, and stale opcodes
    // surviving a cartridge data access. No earlier test enables prefetch.
    #[test]
    fn prefetch_consumes_halfwords_and_resets_on_cartridge_data() {
        let mut bus = System::new();
        let fetch = Access {
            kind: AccessKind::Fetch,
            sequential: true,
        };
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        bus.write16_impl(IO_START + 0x204, 0x4000, data).unwrap();
        bus.charge(
            ROM_START,
            2,
            Access {
                sequential: false,
                ..fetch
            },
        );
        bus.idle(2);
        let start = bus.cycles;
        bus.charge(ROM_START + 2, 2, fetch);
        assert_eq!(bus.cycles - start, 1);
        bus.idle(24);
        let start = bus.cycles;
        bus.charge(ROM_START + 4, 4, fetch);
        assert_eq!(bus.cycles - start, 1);
        bus.charge(ROM_START + 0x100, 2, data);
        let start = bus.cycles;
        bus.charge(ROM_START + 8, 2, fetch);
        assert_eq!(bus.cycles - start, 5);
        bus.write16_impl(IO_START + 0x204, 0, data).unwrap();
        bus.idle(24);
        let start = bus.cycles;
        bus.charge(ROM_START + 10, 2, fetch);
        assert_eq!(bus.cycles - start, 3);
    }

    // Catches stale ARM pipeline words after BX, incorrect Thumb PC alignment,
    // and a BL return that loses its Thumb tag before the final ARM return.
    // Earlier guest fixtures execute exclusively in ARM state.
    #[test]
    fn counter_returns_from_thumb_with_architectural_pc_values() {
        let mut machine = Machine::new();
        machine
            .load_rom(include_bytes!("../../../roms/counter.gba"))
            .unwrap();
        machine.set_button(Button::A, true);
        let report = machine
            .run_until_pc(0x08000040, 100_000, Cycle(300_000))
            .unwrap();
        assert_eq!(report.instruction_address, 0x08000040);
        assert_eq!(machine.cpu.cpsr & CPSR_T, 0);
        assert_eq!(machine.cpu.registers[15], 0x08000044);
        assert_eq!(machine.cpu.registers[0], 1);
        assert_eq!(machine.cpu.registers[2], 0x080000d0);
        assert_eq!(machine.cpu.registers[4], 0x080000b4);
        assert_eq!(machine.cpu.registers[14], 0x080000bf);
    }

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
    fn vcount_irq_latches_once_per_compare_edge() {
        let mut machine = Machine::new();

        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };

        // VCOUNT compare = 150, VCOUNT IRQ enabled.
        machine
            .system
            .write16_impl(IO_START + 4, (150 << 8) | 0x20, access)
            .unwrap();

        // Enable VCOUNT in IE.
        machine
            .system
            .write16_impl(IO_START + 0x200, 1 << 2, access)
            .unwrap();

        let first_edge = 150 * CYCLES_PER_SCANLINE;

        assert!(machine.cycles().0 < first_edge);

        // Stop immediately before the comparison edge.
        machine
            .system
            .advance_time(first_edge - machine.cycles().0 - 1);

        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2), 0,);

        // Enter scanline 150.
        machine.system.advance_time(1);

        assert_eq!(machine.inspect16(IO_START + 6).unwrap(), 150,);

        assert_ne!(
            machine.inspect16(IO_START + 4).unwrap() & 0x04,
            0,
            "VCOUNT comparison status must be set",
        );

        assert_eq!(
            machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
            1 << 2,
            "VCOUNT must latch IF bit 2",
        );

        // Guest acknowledges VCOUNT.
        machine
            .system
            .write16_impl(IO_START + 0x202, 1 << 2, access)
            .unwrap();

        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2), 0,);

        // Remaining inside line 150 must NOT continuously regenerate the IRQ.
        machine.system.advance_time(100);

        assert_eq!(
            machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
            0,
            "VCOUNT IRQ must trigger only on the compare edge",
        );

        // It should fire again on line 150 of the following frame.
        let second_edge = CYCLES_PER_FRAME + first_edge;

        machine
            .system
            .advance_time(second_edge - machine.cycles().0);

        assert_eq!(
            machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
            1 << 2,
        );
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

    // Catches missing FIFO refill, DMA source rewind and sample production tied
    // to host deadlines. Existing DMA3 image tests do not consume audio FIFOs.
    #[test]
    fn timer_fifo_dma_sound_is_identical_across_bounded_advances() {
        let run = |chunk: u64, fifo: u32, bias: u16, channel: usize| {
            let mut machine = Machine::new();
            machine.system.halted = true;
            for (index, byte) in machine.system.ewram.iter_mut().take(128).enumerate() {
                *byte = (index as u8).wrapping_mul(3);
            }
            let data = Access {
                kind: AccessKind::Data,
                sequential: false,
            };
            machine
                .system
                .write32_impl(IO_START + 0xb0 + channel as u32 * 12, EWRAM_START, data)
                .unwrap();
            machine
                .system
                .write32_impl(IO_START + 0xb4 + channel as u32 * 12, IO_START + fifo, data)
                .unwrap();
            // FIFO mode forces four words and a fixed destination regardless of count/width.
            machine
                .system
                .write32_impl(IO_START + 0xb8 + channel as u32 * 12, 0xb2000001, data)
                .unwrap();
            machine
                .system
                .write16_impl(IO_START + 0x100, 0xfe00, data)
                .unwrap();
            machine
                .system
                .write16_impl(IO_START + 0x102, 0xc0, data)
                .unwrap();
            machine
                .system
                .write16_impl(IO_START + 0x84, 0x80, data)
                .unwrap();
            machine
                .system
                .write16_impl(
                    IO_START + 0x82,
                    if fifo == 0xa0 { 0x304 } else { 0x3008 },
                    data,
                )
                .unwrap();
            machine
                .system
                .write16_impl(IO_START + 0x88, bias, data)
                .unwrap();
            let end = machine.cycles().0 + 48 * 512;
            let mut pcm = Vec::new();
            while machine.cycles().0 < end {
                machine
                    .advance_to(Cycle((machine.cycles().0 + chunk).min(end)), 1)
                    .unwrap();
                machine.drain_pcm(&mut pcm);
            }
            assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 8, 8);
            assert_eq!(pcm.len(), end as usize / 512);
            assert!(pcm.iter().any(|&sample| sample > 0.5));
            assert!(pcm.iter().any(|&sample| sample < -0.5));
            pcm
        };
        // DMA2 previously never enabled or refilled. Both channels, FIFO
        // destinations and PWM cadences must remain independent of
        // host chunk size, including chunks that split a PWM sample interval.
        for fifo in [0xa0, 0xa4] {
            for bias in [0x200, 0x4200, 0x8200, 0xc200] {
                for channel in [1, 2] {
                    assert_eq!(run(37, fifo, bias, channel), run(4096, fifo, bias, channel));
                }
            }
        }
    }

    #[test]
    fn dma_upload_resumes_at_deadlines_and_latches_vblank_irq() {
        // A large immediate upload must relinquish the host at a beat boundary,
        // without retiring CPU instructions or skipping the intervening IRQ edge.
        let mut machine = Machine::new();
        machine.system.halted = true;
        machine.system.ewram.fill(0x5a);
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine.system.write16_impl(IO_START + 4, 8, data).unwrap();
        machine
            .system
            .advance_time(160 * CYCLES_PER_SCANLINE - 20 - machine.cycles().0);
        machine
            .system
            .write32_impl(IO_START + 0xd4, EWRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xd8, VRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xdc, 0x8000_4000, data)
            .unwrap();
        let start = machine.cycles().0;
        machine.advance_to(Cycle(start + 8), 1).unwrap();
        assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x5a5a);
        assert_eq!(machine.inspect16(VRAM_START + 32766).unwrap(), 0);
        assert_eq!(machine.inspect16(IO_START + 0xde).unwrap() & 0x8000, 0x8000);
        assert_eq!(machine.executed_instructions(), 0);
        assert!(machine.cycles().0 <= start + 12);
        machine.advance_to(Cycle(start + 70000), 1).unwrap();
        assert_eq!(machine.inspect16(VRAM_START + 32766).unwrap(), 0x5a5a);
        assert_eq!(machine.inspect16(IO_START + 0xde).unwrap() & 0x8000, 0);
        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 1, 1);
    }

    #[test]
    fn repeating_dma_reloads_destination_without_rewinding_source() {
        // Repeated HBlank uploads must progress through successive source rows.
        // Reloading both addresses would silently display the first row forever.
        let mut machine = Machine::new();
        machine.system.halted = true;
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        for (offset, value) in [(0, 0x11223344), (4, 0x55667788)] {
            machine
                .system
                .write32_impl(EWRAM_START + offset, value, data)
                .unwrap();
        }
        machine
            .system
            .write32_impl(IO_START + 0xd4, EWRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xd8, VRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xdc, 0xa6600001, data)
            .unwrap();
        machine.advance_to(Cycle(HBLANK_FLAG_CYCLE - 1), 1).unwrap();
        assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0);
        machine
            .advance_to(Cycle(HBLANK_FLAG_CYCLE + 20), 1)
            .unwrap();
        assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x3344);
        assert_eq!(machine.inspect16(VRAM_START + 2).unwrap(), 0x1122);
        machine
            .advance_to(Cycle(CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE + 20), 1)
            .unwrap();
        assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x7788);
        assert_eq!(machine.inspect16(VRAM_START + 2).unwrap(), 0x5566);
        assert_eq!(machine.inspect16(VRAM_START + 4).unwrap(), 0);
        machine
            .system
            .write16_impl(IO_START + 0xde, 0, data)
            .unwrap();
        machine
            .advance_to(Cycle(3 * CYCLES_PER_SCANLINE), 1)
            .unwrap();
        assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x7788);
    }

    #[test]
    fn dma0_hblank_repeat_reloads_destination_and_skips_vblank() {
        let mut machine = Machine::new();

        // HALT isolates scheduler wakeups from CPU instruction execution.
        machine.system.halted = true;

        // Each visible HBlank consumes one four-byte pair of register values.
        for index in 0..160usize {
            let value = u16::try_from(index).unwrap();
            let word = u32::from(0x0404u16) | (u32::from(value) << 16);
            let offset = index * 4;
            machine.system.ewram[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
        }

        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine
            .system
            .write32_impl(IO_START + 0xb0, EWRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xb4, IO_START + 0x52, data)
            .unwrap();
        // Count=2 halfwords; HBlank timing, repeat, and destination reload.
        machine
            .system
            .write32_impl(IO_START + 0xb8, (u32::from(0xa260u16) << 16) | 2, data)
            .unwrap();

        machine
            .advance_to(Cycle(HBLANK_FLAG_CYCLE + 32), 1)
            .unwrap();

        assert_eq!(machine.inspect16(IO_START + 0x52).unwrap(), 0x0404);
        assert_eq!(machine.inspect16(IO_START + 0x54).unwrap(), 0);
        assert_eq!(machine.system.dma[0].source, EWRAM_START + 4);
        assert_eq!(machine.system.dma[0].destination, IO_START + 0x52);
        assert_eq!(machine.executed_instructions(), 0);

        machine
            .advance_to(Cycle(CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE + 32), 1)
            .unwrap();

        assert_eq!(machine.inspect16(IO_START + 0x54).unwrap(), 1);
        assert_eq!(machine.system.dma[0].source, EWRAM_START + 8);
        assert_eq!(machine.system.dma[0].destination, IO_START + 0x52);

        // Exactly 160 visible requests consume 160 source entries.
        machine.advance_to(Cycle(CYCLES_PER_FRAME - 1), 1).unwrap();
        assert_eq!(machine.system.dma[0].source, EWRAM_START + 160 * 4);
        let source_after_visible = machine.system.dma[0].source;

        // HBlank during VBlank must not advance the repeated source table.
        machine
            .advance_to(Cycle(CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE - 1), 1)
            .unwrap();
        assert_eq!(machine.system.dma[0].source, source_after_visible);

        // Repeated transfers resume on the next visible line-zero HBlank.
        machine
            .advance_to(Cycle(CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE + 32), 1)
            .unwrap();
        assert_eq!(machine.system.dma[0].source, source_after_visible + 4);
    }

    #[test]
    fn simultaneous_hblank_dma_obeys_channel_priority() {
        let mut machine = Machine::new();
        let dma0_source = EWRAM_START + 0x0800;
        let dma3_source = EWRAM_START + 0x0900;
        let destination = EWRAM_START + 0x1000;
        machine.system.ewram[0x0800..0x0802].copy_from_slice(&0x1111u16.to_le_bytes());
        machine.system.ewram[0x0900..0x0902].copy_from_slice(&0x3333u16.to_le_bytes());

        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };

        // Arm the lower-priority channel first to prove enable order is irrelevant.
        machine
            .system
            .write32_impl(IO_START + 0xd4, dma3_source, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xd8, destination, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xdc, (u32::from(0xa240u16) << 16) | 1, data)
            .unwrap();

        machine
            .system
            .write32_impl(IO_START + 0xb0, dma0_source, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xb4, destination, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xb8, (u32::from(0xa240u16) << 16) | 1, data)
            .unwrap();

        let edge = machine.system.next_visible_hblank();
        machine.system.advance_time(edge - machine.system.cycles);
        assert!(machine.system.dma[0].active);
        assert!(machine.system.dma[3].active);

        let dma3_before = machine.system.dma[3].source;
        machine.step().unwrap();
        assert_eq!(machine.inspect16(destination).unwrap(), 0x1111);
        assert!(!machine.system.dma[0].active);
        assert!(machine.system.dma[3].active);
        assert_eq!(machine.system.dma[3].source, dma3_before);

        machine.step().unwrap();
        assert_eq!(machine.inspect16(destination).unwrap(), 0x3333);
        assert!(!machine.system.dma[3].active);
        assert_eq!(machine.executed_instructions(), 0);
    }

    #[test]
    fn dma0_latches_14_bit_count_and_27_bit_addresses() {
        let mut system = System::new();
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };

        system
            .write32_impl(IO_START + 0xb0, 0x0fff_fffd, data)
            .unwrap();
        system
            .write32_impl(IO_START + 0xb4, 0x0fff_fffd, data)
            .unwrap();
        // DMA0's 14-bit CNT_L turns 0x4001 into one transfer.
        system
            .write32_impl(IO_START + 0xb8, (u32::from(0xa000u16) << 16) | 0x4001, data)
            .unwrap();

        assert_eq!(system.dma[0].source, 0x07ff_fffc);
        assert_eq!(system.dma[0].destination, 0x07ff_fffc);
        assert_eq!(system.dma[0].count, 1);
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

    #[test]
    fn unchanged_button_state_does_not_retrigger_keypad_irq() {
        let mut machine = Machine::new();
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };

        // Select A, OR mode, IRQ enabled.
        machine
            .system
            .write16_impl(IO_START + 0x132, 0x4001, access)
            .unwrap();

        machine.set_button(Button::A, true);
        assert_eq!(
            machine.inspect16(IO_START + 0x202).unwrap() & 0x1000,
            0x1000
        );

        // Acknowledge keypad IF.
        machine
            .system
            .write16_impl(IO_START + 0x202, 0x1000, access)
            .unwrap();

        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 0x1000, 0);

        // Host polling the same state repeatedly must have no hardware effect.
        for _ in 0..100 {
            machine.set_button(Button::A, true);
        }

        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 0x1000, 0);

        // A genuine transition may request it again.
        machine.set_button(Button::A, false);
        machine.set_button(Button::A, true);

        assert_eq!(
            machine.inspect16(IO_START + 0x202).unwrap() & 0x1000,
            0x1000
        );
    }

    #[test]
    fn firered_agbprint_init_rom_writes_do_not_fault_or_mutate_rom() {
        let mut bus = System::new();
        bus.rom = vec![0x5a; 16 * 1024 * 1024];

        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };

        let original_prefix = bus.rom[..16].to_vec();

        // Retail FireRed Rev 0 retained AGBPrintInit. These are cartridge-ROM
        // writes used by that SDK debug facility. On an ordinary cartridge they
        // must not raise a CPU-visible memory fault.
        for (address, value) in [
            (0x09fe_2ffe, 0x0020),
            (0x09fe_20f8, 0x0000),
            (0x09fe_20fc, 0x0000),
            (0x09fe_20fe, 0x0000),
            (0x09fe_20fa, 0x00fd),
            (0x09fe_2ffe, 0x0000),
        ] {
            bus.write16_impl(address, value, access).unwrap();
        }

        assert_eq!(&bus.rom[..16], original_prefix.as_slice());

        // Ordinary in-range ROM is equally read-only.
        bus.write16_impl(ROM_START, 0x1234, access).unwrap();

        assert_eq!(&bus.rom[..16], original_prefix.as_slice());
    }

    #[test]
    fn blend_targets_use_the_immediate_visible_second_surface() {
        let mut io = [0u8; IO_BYTES];

        // BG0 first target, BG1 second target, alpha mode.
        io[0x50..0x52].copy_from_slice(&0x0241u16.to_le_bytes());

        // Values above 16 clamp to 16 when used.
        io[0x52..0x54].copy_from_slice(&0x1f1fu16.to_le_bytes());

        let effects = ColorEffects::from_io(&io);

        assert_eq!(effects.eva, 16);
        assert_eq!(effects.evb, 16);

        let red = LayerPixel {
            color: 0x001f,
            layer: 0,
            priority: 0,
            semitransparent: false,
        };

        let blue = LayerPixel {
            color: 0x7c00,
            layer: 1,
            priority: 1,
            semitransparent: false,
        };

        let green = LayerPixel {
            color: 0x03e0,
            layer: 2,
            priority: 0,
            semitransparent: false,
        };

        let mut direct = PixelStack::new(0);
        direct.insert(blue);
        direct.insert(red);

        // 16/16 + 16/16 saturates red and blue.
        assert_eq!(effects.resolve(direct, true), 0x7c1f);

        let mut blocked = PixelStack::new(0);
        blocked.insert(blue);
        blocked.insert(green);
        blocked.insert(red);

        /*
         * BG2 is immediately below BG0 but is not selected as target 2.
         * Hardware must not skip BG2 and blend against deeper BG1.
         */
        assert_eq!(effects.resolve(blocked, true), 0x001f);
    }

    #[test]
    fn semitransparent_obj_forces_alpha_before_window_effect_gate() {
        let effects = ColorEffects {
            // Ordinary fallback selects OBJ for darkening.
            first: 1 << BLEND_LAYER_OBJ,

            // Only BG0 is eligible beneath the semitransparent OBJ.
            second: 1 << 0,

            mode: 3,
            eva: 8,
            evb: 8,
            evy: 8,
        };

        let mut stack = PixelStack::new(0);

        stack.insert(LayerPixel {
            color: 0x001f,
            layer: 0,
            priority: 0,
            semitransparent: false,
        });

        stack.insert(LayerPixel {
            color: 0x7fff,
            layer: BLEND_LAYER_OBJ,
            priority: 0,
            semitransparent: true,
        });

        /*
         * Forced semitransparent alpha wins over BLDCNT darken and occurs even
         * when the ordinary window special-effects bit is clear.
         */
        assert_eq!(effects.resolve(stack, false), 0x41ff);

        let no_second_target = ColorEffects {
            second: 0,
            ..effects
        };

        // No eligible second surface and SFX disabled: unchanged.
        assert_eq!(no_second_target.resolve(stack, false), 0x7fff);

        // Without a forced blend source, normal darken behavior is allowed.
        assert_eq!(no_second_target.resolve(stack, true), 0x41f0);
    }

    #[test]
    fn color_effect_arithmetic_matches_hardware_rounding_and_green_precision() {
        // Nearest-rounded 50/50 red + blue.
        assert_eq!(ColorEffects::alpha_blend(0x001f, 0x7c00, 8, 8), 0x4010);

        // RGB555 bit 15 contributes the hidden sixth green precision bit.
        assert_eq!(ColorEffects::alpha_blend(0x8000, 0x03e0, 8, 8), 0x0200);

        assert_eq!(ColorEffects::brighten(0x001f, 8), 0x421f);

        assert_eq!(ColorEffects::darken(0x001f, 8), 0x0010);
    }
}
