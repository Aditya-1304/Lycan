//! Channel 3 owns two banks of packed, high-nibble-first four-bit samples.
//! CPU accesses always address the bank opposite the current playback bank.

/// Master-clock wave oscillator. RAM survives master disable; control and
/// divider state reset with the PSG. PCM drains never alter oscillator history.
#[derive(Default)]
pub(super) struct Wave {
    pub select: u16,
    pub volume: u16,
    pub control: u16,
    pub active: bool,
    ram: [[u8; 16]; 2],
    bank: usize,
    position: usize,
    phase: u64,
    length: u16,
}

impl Wave {
    fn period(&self) -> u64 {
        8 * (2048 - u64::from(self.control & 0x7ff))
    }

    /// Latch register writes and self-clearing restart. Selecting a bank also
    /// selects the CPU-visible opposite bank while playback is stopped.
    pub fn write(&mut self, offset: usize, value: u16) {
        match offset {
            0x70 => {
                self.select = value & 0xe0;
                self.bank = usize::from(value & 0x40 != 0);
                if value & 0x80 == 0 {
                    self.active = false;
                }
            }
            0x72 => {
                self.volume = value & 0xe0ff;
                self.length = 256 - (value & 0xff);
            }
            0x74 => {
                self.control = value & 0x47ff;
                self.phase %= self.period();
                if value & 0x8000 != 0 {
                    self.active = self.select & 0x80 != 0;
                    self.position = 0;
                    self.phase = 0;
                    self.bank = usize::from(self.select & 0x40 != 0);
                    if self.length == 0 {
                        self.length = 256;
                    }
                }
            }
            0x90..=0x9e => {
                let index = offset - 0x90;
                self.ram[self.bank ^ 1][index..index + 2].copy_from_slice(&value.to_le_bytes());
            }
            _ => {}
        }
    }

    /// Readback exposes the live CPU bank and masks write-only length/frequency.
    pub fn refresh(&self, io: &mut [u8]) {
        for (offset, value) in [
            (0x70, (self.select & !0x40) | ((self.bank as u16) << 6)),
            (0x72, self.volume & 0xe000),
            (0x74, self.control & 0x4000),
        ] {
            io[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        io[0x90..0xa0].copy_from_slice(&self.ram[self.bank ^ 1]);
    }

    pub fn reset(&mut self) {
        let ram = self.ram;
        *self = Self {
            ram,
            ..Self::default()
        };
    }

    pub fn sequence(&mut self, step: u8) {
        if step & 1 == 0 && self.control & 0x4000 != 0 && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
            }
        }
    }

    pub fn next_edge(&self) -> Option<u64> {
        self.active.then(|| self.period() - self.phase)
    }

    /// The event scheduler visits every nibble boundary. In 64-sample mode the
    /// bank toggles after 32 digits, changing CPU RAM access at the same edge.
    pub fn advance(&mut self, elapsed: u64) {
        if self.active {
            let total = self.phase + elapsed;
            let period = self.period();
            // Ordinary device advancement reaches one nibble edge at most.
            // Preserve multi-edge advancement for standalone device callers.
            let (steps, phase) = if total < period {
                (0, total)
            } else if total - period < period {
                (1, total - period)
            } else {
                (total / period, total % period)
            };
            self.phase = phase;
            let position = self.position + steps as usize;
            if self.select & 0x20 != 0 && (position / 32) & 1 != 0 {
                self.bank ^= 1;
                self.select ^= 0x40;
            }
            self.position = position & 31;
        }
    }

    pub fn level(&self) -> i32 {
        if !self.active {
            return 0;
        }
        let byte = self.ram[self.bank][self.position / 2];
        let sample = if self.position & 1 == 0 {
            byte >> 4
        } else {
            byte & 15
        };
        let level = i32::from(sample) * 2 - 15;
        if self.volume & 0x8000 != 0 {
            level * 3 / 4
        } else {
            match (self.volume >> 13) & 3 {
                1 => level,
                2 => level / 2,
                3 => level / 4,
                _ => 0,
            }
        }
    }
}
