//! Host-independent Flash command decoder and revision-tagged backup bytes.
//! Commands complete synchronously; physical busy timing is not modeled here.

use crate::{SaveImage, SaveStatus};

/// Physical backup capacity, also used to validate persisted and imported images.
pub const FLASH64_BYTES: usize = 64 * 1024;
/// Two independently selected 64 KiB banks, persisted as one physical image.
pub const FLASH128_BYTES: usize = 128 * 1024;

/// Only uninterrupted unlock writes can authorize a destructive operation.
#[derive(Clone, Copy)]
enum Command {
    Idle,
    Unlock,
    Select,
    Program,
    Bank,
    EraseUnlock,
    EraseSelect,
    Erase,
}

/// Cartridge-owned state. Host storage snapshots bytes and revisions; command
/// progress and identification mode are volatile and never enter the save image.
pub(crate) struct Flash {
    bytes: Vec<u8>,
    bank: usize,
    command: Command,
    identification: bool,
    revision: u64,
    acknowledged: u64,
}

impl Flash {
    pub fn new(banked: bool) -> Self {
        Self {
            bytes: vec![
                0xff;
                if banked {
                    FLASH128_BYTES
                } else {
                    FLASH64_BYTES
                }
            ],
            bank: 0,
            command: Command::Idle,
            identification: false,
            revision: 0,
            acknowledged: 0,
        }
    }

    /// Reading data abandons a partial command without leaving identification
    /// mode. Debug inspection remains separate from guest bus accesses.
    pub fn read(&mut self, address: u32) -> u8 {
        self.command = Command::Idle;
        let offset = address as usize & (FLASH64_BYTES - 1);
        if self.identification {
            match offset {
                0 => {
                    if self.bytes.len() == FLASH128_BYTES {
                        0x62
                    } else {
                        0x32
                    }
                } // Sanyo / Panasonic.
                1 => {
                    if self.bytes.len() == FLASH128_BYTES {
                        0x13
                    } else {
                        0x1b
                    }
                } // 128 / 64 KiB device.
                _ => self.bytes[self.bank * FLASH64_BYTES + offset],
            }
        } else {
            self.bytes[self.bank * FLASH64_BYTES + offset]
        }
    }

    /// The eight-bit save bus supplies one command or data byte per access.
    /// Programming clears bits; only an erase can return them to one.
    pub fn write(&mut self, address: u32, value: u8) {
        let offset = address as usize & (FLASH64_BYTES - 1);
        let state = std::mem::replace(&mut self.command, Command::Idle);
        if matches!(state, Command::Program) {
            self.replace_byte(
                self.bank * FLASH64_BYTES + offset,
                self.bytes[self.bank * FLASH64_BYTES + offset] & value,
            );
            return;
        }
        if matches!(state, Command::Bank) {
            // Invalid selectors terminate the command without changing banks.
            if offset == 0 && value <= 1 {
                self.bank = usize::from(value);
            }
            return;
        }
        if value == 0xf0 {
            self.identification = false;
            return;
        }
        let next = match state {
            Command::Idle if offset == 0x5555 && value == 0xaa => Command::Unlock,
            Command::Unlock if offset == 0x2aaa && value == 0x55 => Command::Select,
            Command::Select if offset == 0x5555 => match value {
                0x90 => {
                    self.identification = true;
                    Command::Idle
                }
                0xa0 => Command::Program,
                0xb0 if self.bytes.len() == FLASH128_BYTES => Command::Bank,
                0x80 => Command::EraseUnlock,
                _ => Command::Idle,
            },
            Command::EraseUnlock if offset == 0x5555 && value == 0xaa => Command::EraseSelect,
            Command::EraseSelect if offset == 0x2aaa && value == 0x55 => Command::Erase,
            Command::Erase => {
                let range = if offset == 0x5555 && value == 0x10 {
                    0..self.bytes.len()
                } else if offset & 0xfff == 0 && value == 0x30 {
                    self.bank * FLASH64_BYTES + offset..self.bank * FLASH64_BYTES + offset + 0x1000
                } else {
                    return;
                };
                let start = range.start;
                let end = range.end;

                if self.bytes[start..end].iter().any(|&byte| byte != 0xff) {
                    self.bytes[start..end].fill(0xff);
                    self.revision += 1;
                }
                Command::Idle
            }
            _ => Command::Idle,
        };
        self.command = next;
    }

    fn replace_byte(&mut self, offset: usize, value: u8) {
        if self.bytes[offset] != value {
            self.bytes[offset] = value;
            self.revision += 1;
        }
    }

    /// Reset volatile protocol state while preserving pending storage revisions.
    pub fn reset(&mut self) {
        self.command = Command::Idle;
        self.identification = false;
        self.bank = 0;
    }

    /// Reports backup metadata without cloning the Flash bytes.
    pub fn status(&self) -> SaveStatus {
        SaveStatus {
            len: self.bytes.len(),
            revision: self.revision,
            dirty: self.revision > self.acknowledged,
        }
    }

    pub fn image(&self) -> SaveImage {
        SaveImage {
            bytes: self.bytes.clone(),
            revision: self.revision,
            dirty: self.revision > self.acknowledged,
        }
    }

    /// Validate before mutation so a rejected host image cannot damage the chip.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() != self.bytes.len() {
            return Err("Flash image size must match the selected cartridge capacity");
        }
        self.bytes.copy_from_slice(bytes);
        self.revision += 1;
        self.acknowledged = self.revision;
        self.reset();
        Ok(())
    }

    pub fn import(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let acknowledged = self.acknowledged;
        self.load(bytes)?;
        self.acknowledged = acknowledged;
        Ok(())
    }

    /// A stale storage completion cannot acknowledge a more recent mutation.
    pub fn acknowledge(&mut self, revision: u64) {
        self.acknowledged = self.acknowledged.max(revision.min(self.revision));
    }
}
