//! Cartridge serial EEPROM. Halfword accesses clock bit zero, MSB first.
//! Programming completes synchronously, matching the existing Flash abstraction.

use crate::{SaveImage, SaveStatus};

/// Physical capacity of a six-address-bit EEPROM.
pub const EEPROM512_BYTES: usize = 512;
/// Physical capacity of a fourteen-address-bit EEPROM.
pub const EEPROM8K_BYTES: usize = 8192;

/// Only bytes and revisions are persistent. Command buffers and read cursors
/// belong to the cartridge protocol and are discarded on reset or host restore.
pub(crate) struct Eeprom {
    bytes: Vec<u8>,
    capacity: Option<usize>,
    address_bits: Option<usize>,
    command: Vec<u8>,
    /// DMA boundaries disambiguate size and reject a command/count mismatch.
    command_length: Option<usize>,
    output: Option<(usize, usize)>,
    revision: u64,
    acknowledged: u64,
}

impl Eeprom {
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            bytes: vec![0xff; capacity.unwrap_or(EEPROM8K_BYTES)],
            capacity,
            address_bits: capacity.map(|size| if size == EEPROM512_BYTES { 6 } else { 14 }),
            command: Vec::with_capacity(81),
            command_length: None,
            output: None,
            revision: 0,
            acknowledged: 0,
        }
    }

    /// SDK signatures identify EEPROM but not its capacity. DMA3 supplies an
    /// unambiguous command boundary: 9/73 halfwords use six address bits;
    /// 17/81 use fourteen. A detected or forced capacity never silently changes.
    pub fn begin_dma(&mut self, count: u32) {
        self.command.clear();
        self.output = None;
        self.command_length = Some(count as usize);
        self.address_bits = match count {
            9 | 73 => Some(6),
            17 | 81 => Some(14),
            _ => None,
        }
        .filter(|&bits| {
            self.capacity.is_none_or(|size| {
                size == if bits == 6 {
                    EEPROM512_BYTES
                } else {
                    EEPROM8K_BYTES
                }
            })
        });
    }

    /// Commit only a complete command with a zero stop bit. Invalid or partial
    /// streams cannot dirty the image or resolve an unknown cartridge capacity.
    pub fn write(&mut self, value: u16) {
        self.output = None;
        let bit = (value & 1) as u8;
        if self.command.is_empty() && bit == 0 {
            return;
        }
        let Some(bits) = self.address_bits else {
            return;
        };
        self.command.push(bit);
        if self.command.len() < 2 {
            return;
        }
        let read = self.command[1] == 1;
        let length = 2 + bits + if read { 0 } else { 64 } + 1;
        if self.command.len() != length {
            return;
        }
        if bit != 0 || self.command_length.is_some_and(|count| count != length) {
            self.command.clear();
            return;
        }
        let size = if bits == 6 {
            EEPROM512_BYTES
        } else {
            EEPROM8K_BYTES
        };
        if self.capacity.is_none() {
            self.capacity = Some(size);
            self.bytes.truncate(size);
        }
        let block = self.command[2..2 + bits]
            .iter()
            .fold(0_usize, |value, &bit| (value << 1) | usize::from(bit));
        // The large device transmits fourteen address bits, but only ten select
        // its 1024 physical blocks. Each block contains eight consecutive bytes.
        let offset = (block & (size / 8 - 1)) * 8;
        if read {
            self.output = Some((offset, 0));
        } else {
            let mut data = [0_u8; 8];
            for (index, &bit) in self.command[2 + bits..length - 1].iter().enumerate() {
                data[index / 8] |= bit << (7 - index % 8);
            }
            if self.bytes[offset..offset + 8] != data {
                self.bytes[offset..offset + 8].copy_from_slice(&data);
                self.revision += 1;
            }
        }
        self.command.clear();
    }

    /// A read request returns four dummy clocks followed by 64 data clocks.
    /// Outside that response the chip reports ready; polling never changes bytes.
    pub fn read(&mut self) -> u16 {
        self.command.clear();
        let Some((offset, cursor)) = self.output else {
            return 1;
        };
        let bit = if cursor < 4 {
            0
        } else {
            let index = cursor - 4;
            (self.bytes[offset + index / 8] >> (7 - index % 8)) & 1
        };
        self.output = if cursor == 67 {
            None
        } else {
            Some((offset, cursor + 1))
        };
        u16::from(bit)
    }

    pub fn reset(&mut self) {
        self.command_length = None;
        self.command.clear();
        self.output = None;
        self.address_bits = self
            .capacity
            .map(|size| if size == EEPROM512_BYTES { 6 } else { 14 });
    }

    /// Reports resolved EEPROM capacity and revision metadata without cloning bytes.
    pub fn status(&self) -> SaveStatus {
        SaveStatus {
            len: self.capacity.unwrap_or(0),
            revision: self.revision,
            dirty: self.revision > self.acknowledged,
        }
    }

    /// An unresolved image is empty and clean. It still exposes the host storage
    /// route so an existing validated image can resolve capacity before execution.
    pub fn image(&self) -> SaveImage {
        SaveImage {
            bytes: self.capacity.map_or_else(Vec::new, |_| self.bytes.clone()),
            revision: self.revision,
            dirty: self.revision > self.acknowledged,
        }
    }

    /// A valid saved image may resolve an unknown device, but cannot replace an
    /// already detected or overridden capacity. Validate before any mutation.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if !matches!(bytes.len(), EEPROM512_BYTES | EEPROM8K_BYTES)
            || self.capacity.is_some_and(|size| size != bytes.len())
        {
            return Err("EEPROM image must match the selected 512-byte or 8192-byte capacity");
        }
        self.capacity = Some(bytes.len());
        self.bytes = bytes.to_vec();
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

    pub fn acknowledge(&mut self, revision: u64) {
        self.acknowledged = self.acknowledged.max(revision.min(self.revision));
    }
}
