//! Cartridge serial EEPROM. Halfword accesses clock bit zero, MSB first.
//! Programming completes synchronously, matching the existing Flash abstraction.

use crate::{SaveImage, SaveStatus};

/// One protocol boundary observed without changing guest execution or save bytes.
#[cfg(feature = "eeprom-trace")]
#[derive(Debug)]
pub struct EepromTrace {
    pub cycle: u64,
    pub event: EepromTraceEvent,
}

/// Transaction-level observations; data bytes retain their serial MSB-first order.
#[cfg(feature = "eeprom-trace")]
#[derive(Debug)]
pub enum EepromTraceEvent {
    Dma {
        source: u32,
        destination: u32,
        count: u32,
        width: u32,
    },
    Command {
        read: bool,
        address_bits: usize,
        raw_block: usize,
        trailing_bit: u8,
        block: usize,
        data: [u8; 8],
    },
    Response {
        block: usize,
        data: [u8; 8],
    },
    Rejected {
        length: usize,
        stop: u8,
        dma_count: Option<usize>,
    },
    Ready,
}

/// The host drains this queue between frames. Overflow is explicit, so an
/// incomplete capture cannot be mistaken for a complete protocol transcript.
#[cfg(feature = "eeprom-trace")]
#[derive(Default)]
struct TraceBuffer {
    events: std::collections::VecDeque<EepromTrace>,
    dropped: u64,
    ready_reported: bool,
}

/// Physical capacity of a six-address-bit EEPROM.
pub const EEPROM512_BYTES: usize = 512;
/// Physical capacity of a fourteen-address-bit EEPROM.
pub const EEPROM8K_BYTES: usize = 8192;

/// Only bytes and revisions are persistent. Command buffers and read cursors
/// belong to the cartridge protocol and are discarded on reset or host restore.
pub(crate) struct Eeprom {
    #[cfg(feature = "eeprom-trace")]
    trace: Option<TraceBuffer>,
    #[cfg(feature = "eeprom-trace")]
    trace_cycle: u64,
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
            #[cfg(feature = "eeprom-trace")]
            trace: None,
            #[cfg(feature = "eeprom-trace")]
            trace_cycle: 0,
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
        #[cfg(feature = "eeprom-trace")]
        if let Some(trace) = &mut self.trace {
            trace.ready_reported = false;
        }
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

    /// Accept only complete commands matching the DMA boundary. Writes require
    /// a zero stop bit; the final read clock is consumed regardless of its value.
    /// Invalid or partial streams cannot dirty the image or resolve capacity.
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
        // Read termination is a clock boundary, not a validated data bit. Retail
        // routines can leave it high; rejecting it substitutes ready bits for
        // the requested block and makes the guest report a corrupt save.
        if (!read && bit != 0) || self.command_length.is_some_and(|count| count != length) {
            #[cfg(feature = "eeprom-trace")]
            self.record(EepromTraceEvent::Rejected {
                length,
                stop: bit,
                dma_count: self.command_length,
            });
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
        #[cfg(feature = "eeprom-trace")]
        {
            let mut data = [0; 8];
            if !read {
                for (index, &bit) in self.command[2 + bits..length - 1].iter().enumerate() {
                    data[index / 8] |= bit << (7 - index % 8);
                }
            }
            self.record(EepromTraceEvent::Command {
                read,
                address_bits: bits,
                raw_block: block,
                trailing_bit: bit,
                block: offset / 8,
                data,
            });
        }
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
            #[cfg(feature = "eeprom-trace")]
            if self
                .trace
                .as_ref()
                .is_some_and(|trace| !trace.ready_reported)
            {
                self.record(EepromTraceEvent::Ready);
                if let Some(trace) = &mut self.trace {
                    trace.ready_reported = true;
                }
            }
            return 1;
        };
        #[cfg(feature = "eeprom-trace")]
        if cursor == 0 {
            let data = self.bytes[offset..offset + 8].try_into().unwrap();
            self.record(EepromTraceEvent::Response {
                block: offset / 8,
                data,
            });
        }
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

    /// Enable a fresh bounded capture for this cartridge only.
    #[cfg(feature = "eeprom-trace")]
    pub fn enable_trace(&mut self) {
        self.trace = Some(TraceBuffer::default());
    }

    /// Update the timestamp at the bus boundary, including charged wait states.
    #[cfg(feature = "eeprom-trace")]
    pub fn trace_cycle(&mut self, cycle: u64) {
        self.trace_cycle = cycle;
    }

    #[cfg(feature = "eeprom-trace")]
    pub fn record(&mut self, event: EepromTraceEvent) {
        if let Some(trace) = &mut self.trace {
            if trace.events.len() == 512 {
                trace.dropped += 1;
            } else {
                trace.events.push_back(EepromTrace {
                    cycle: self.trace_cycle,
                    event,
                });
            }
        }
    }

    /// Drain retained events and report records omitted since the previous drain.
    #[cfg(feature = "eeprom-trace")]
    pub fn drain_trace(&mut self) -> (Vec<EepromTrace>, u64) {
        self.trace.as_mut().map_or_else(
            || (Vec::new(), 0),
            |trace| {
                (
                    trace.events.drain(..).collect(),
                    std::mem::take(&mut trace.dropped),
                )
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Retail EEPROM routines may leave the final read-command bit high. Rejecting
    /// that clock turns the response into ready bits and breaks save verification;
    /// the existing DMA round-trip test only exercises a zero trailing bit.
    #[test]
    fn read_command_accepts_high_trailing_clock_without_changing_save() {
        for (capacity, address_bits, block) in [(EEPROM512_BYTES, 6, 3), (EEPROM8K_BYTES, 14, 797)]
        {
            let mut chip = Eeprom::new(Some(capacity));
            let mut saved = vec![0xff; capacity];
            let data = [0xfe, 0xbc, 0, 0, 0, 0, 0xda, 0x69];
            saved[block * 8..block * 8 + 8].copy_from_slice(&data);
            chip.load(&saved).unwrap();
            let revision = chip.status().revision;
            chip.begin_dma((2 + address_bits + 1) as u32);
            chip.write(1);
            chip.write(1);
            for shift in (0..address_bits).rev() {
                chip.write(((block >> shift) & 1) as u16);
            }
            chip.write(1);
            for _ in 0..4 {
                assert_eq!(chip.read(), 0, "read response must start with dummy clocks");
            }
            let mut actual = [0; 8];
            for byte in &mut actual {
                for _ in 0..8 {
                    *byte = (*byte << 1) | chip.read() as u8;
                }
            }
            assert_eq!(actual, data);
            assert_eq!(chip.read(), 1, "completed response returns to ready");
            assert_eq!(chip.image().bytes, saved);
            assert_eq!(chip.status().revision, revision);
            assert!(!chip.status().dirty);
        }
    }
}
