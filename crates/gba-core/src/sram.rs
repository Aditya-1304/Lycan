//! Host-independent 32 KiB SRAM and revision-based storage acknowledgement.

pub const SRAM_BYTES: usize = 32 * 1024;

/// Immutable storage work item. Completion acknowledges this revision only.
#[derive(Clone)]
pub struct SaveImage {
    pub bytes: Vec<u8>,
    pub revision: u64,
    pub dirty: bool,
}

/// Lightweight backup metadata for polling without copying the cartridge bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveStatus {
    /// Backup capacity in bytes, or zero while EEPROM capacity is unresolved.
    pub len: usize,
    /// Revision tag used to associate storage acknowledgements with mutations.
    pub revision: u64,
    /// Whether this revision is newer than the latest acknowledged revision.
    pub dirty: bool,
}

pub(crate) struct Sram {
    bytes: Vec<u8>,
    revision: u64,
    acknowledged: u64,
}

impl Sram {
    pub fn new() -> Self {
        Self {
            bytes: vec![0xff; SRAM_BYTES],
            revision: 0,
            acknowledged: 0,
        }
    }

    pub fn read(&self, address: u32) -> u8 {
        self.bytes[address as usize & (SRAM_BYTES - 1)]
    }

    pub fn write(&mut self, address: u32, value: u8) {
        let byte = &mut self.bytes[address as usize & (SRAM_BYTES - 1)];
        if *byte != value {
            *byte = value;
            self.revision += 1;
        }
    }

    /// Reports backup metadata without cloning the SRAM bytes.
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

    pub fn load(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() != SRAM_BYTES {
            return Err("SRAM image must contain exactly 32768 bytes");
        }
        self.bytes.copy_from_slice(bytes);
        self.revision += 1;
        self.acknowledged = self.revision;
        Ok(())
    }

    pub fn import(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let acknowledged = self.acknowledged;
        self.load(bytes)?;
        self.acknowledged = acknowledged;
        Ok(())
    }

    pub fn acknowledge(&mut self, revision: u64) {
        // Out-of-order or invalid completions cannot clean bytes they did not store.
        self.acknowledged = self.acknowledged.max(revision.min(self.revision));
    }
}
