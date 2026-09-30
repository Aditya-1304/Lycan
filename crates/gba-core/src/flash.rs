//! Host-independent Flash64 command decoder and revision-tagged backup bytes.
//! Commands complete synchronously; physical busy timing is not modeled here.

use crate::SaveImage;

/// Physical backup capacity, also used to validate persisted and imported images.
pub const FLASH64_BYTES: usize = 64 * 1024;

/// Only uninterrupted unlock writes can authorize a destructive operation.
#[derive(Clone, Copy)]
enum Command {
    Idle,
    Unlock,
    Select,
    Program,
    EraseUnlock,
    EraseSelect,
    Erase,
}

/// Cartridge-owned state. Host storage snapshots bytes and revisions; command
/// progress and identification mode are volatile and never enter the save image.
pub(crate) struct Flash64 {
    bytes: Vec<u8>,
    command: Command,
    identification: bool,
    revision: u64,
    acknowledged: u64,
}

impl Flash64 {
    pub fn new() -> Self {
        Self {
            bytes: vec![0xff; FLASH64_BYTES],
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
                0 => 0x32, // Panasonic manufacturer.
                1 => 0x1b, // Panasonic 64 KiB device.
                _ => self.bytes[offset],
            }
        } else {
            self.bytes[offset]
        }
    }

    /// The eight-bit save bus supplies one command or data byte per access.
    /// Programming clears bits; only an erase can return them to one.
    pub fn write(&mut self, address: u32, value: u8) {
        let offset = address as usize & (FLASH64_BYTES - 1);
        let state = std::mem::replace(&mut self.command, Command::Idle);
        if matches!(state, Command::Program) {
            self.replace_byte(offset, self.bytes[offset] & value);
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
                0x80 => Command::EraseUnlock,
                _ => Command::Idle,
            },
            Command::EraseUnlock if offset == 0x5555 && value == 0xaa => Command::EraseSelect,
            Command::EraseSelect if offset == 0x2aaa && value == 0x55 => Command::Erase,
            Command::Erase => {
                let range = if offset == 0x5555 && value == 0x10 {
                    0..FLASH64_BYTES
                } else if offset & 0xfff == 0 && value == 0x30 {
                    offset..offset + 0x1000
                } else {
                    return;
                };
                if self.bytes[range.clone()].iter().any(|&byte| byte != 0xff) {
                    self.bytes[range].fill(0xff);
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
        if bytes.len() != FLASH64_BYTES {
            return Err("Flash64 image must contain exactly 65536 bytes");
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
