//! Load and bounded-execution failures exposed by the core API.

use crate::{BIOS_SIZE, Cycle};
use std::{error::Error, fmt};

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
